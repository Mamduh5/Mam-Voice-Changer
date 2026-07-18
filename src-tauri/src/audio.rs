use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{traits::{Consumer, Producer, Split}, HeapRb};
use serde::{Deserialize, Serialize};
use std::sync::{atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering}, Arc, RwLock};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AudioError { #[error("Audio device not found: {0}")] Device(String), #[error("Unsupported audio configuration: {0}")] Config(String), #[error("Audio stream error: {0}")] Stream(String), #[error("Invalid parameter: {0}")] Parameter(String) }

#[derive(Clone, Serialize)] pub struct DeviceInfo { id:String, name:String, is_default:bool }
#[derive(Serialize)] pub struct DeviceList { inputs:Vec<DeviceInfo>, outputs:Vec<DeviceInfo> }
#[derive(Clone, Copy, Deserialize)] pub struct Parameters { input_gain:f32, output_gain:f32, gate_threshold_db:f32, pitch_semitones:f32, mix:f32 }
impl Parameters { pub fn validate(&self)->Result<(),AudioError>{if !(0.0..=2.0).contains(&self.input_gain)||!(0.0..=2.0).contains(&self.output_gain)||!(-70.0..=-20.0).contains(&self.gate_threshold_db)||!(-8.0..=8.0).contains(&self.pitch_semitones)||!(0.0..=1.0).contains(&self.mix){return Err(AudioError::Parameter("one or more values are outside the supported range".into()))}Ok(())}}
impl Default for Parameters { fn default()->Self{Self{input_gain:1.,output_gain:1.,gate_threshold_db:-50.,pitch_semitones:0.,mix:1.}} }
#[derive(Serialize)] pub struct EngineStatus { running:bool,input_level:f32,output_level:f32,underruns:u64,message:String }

struct Metrics { running:AtomicBool,input:AtomicU32,output:AtomicU32,underruns:AtomicU64 }
impl Default for Metrics {fn default()->Self{Self{running:AtomicBool::new(false),input:AtomicU32::new(0),output:AtomicU32::new(0),underruns:AtomicU64::new(0)}}}
pub struct AudioEngine { input:Option<cpal::Stream>,output:Option<cpal::Stream>,parameters:Arc<RwLock<Parameters>>,metrics:Arc<Metrics> }
impl Default for AudioEngine {fn default()->Self{Self{input:None,output:None,parameters:Arc::new(RwLock::new(Parameters::default())),metrics:Arc::new(Metrics::default())}}}

fn devices(input:bool)->Result<Vec<cpal::Device>,AudioError>{let host=cpal::default_host();let iter=if input{host.input_devices()}else{host.output_devices()}.map_err(|e|AudioError::Device(e.to_string()))?;Ok(iter.collect())}
fn device_id(d:&cpal::Device,index:usize)->String{format!("{}:{}",index,d.name().unwrap_or_else(|_|"Unknown device".into()))}
pub fn list_devices()->Result<DeviceList,AudioError>{let host=cpal::default_host();let di=host.default_input_device().and_then(|d|d.name().ok());let do_=host.default_output_device().and_then(|d|d.name().ok());let map=|list:Vec<cpal::Device>,default:Option<String>|list.into_iter().enumerate().map(|(i,d)|{let name=d.name().unwrap_or_else(|_|"Unknown device".into());DeviceInfo{id:device_id(&d,i),is_default:default.as_ref()==Some(&name),name}}).collect();Ok(DeviceList{inputs:map(devices(true)?,di),outputs:map(devices(false)?,do_)})}
fn find_device(input:bool,id:&str)->Result<cpal::Device,AudioError>{devices(input)?.into_iter().enumerate().find(|(i,d)|device_id(d,*i)==id).map(|(_,d)|d).ok_or_else(||AudioError::Device(id.into()))}

impl AudioEngine {
 pub fn start(&mut self,input_id:&str,output_id:&str,p:Parameters)->Result<(),AudioError>{self.stop();p.validate()?;*self.parameters.write().unwrap()=p;let input=find_device(true,input_id)?;let output=find_device(false,output_id)?;let ic=input.default_input_config().map_err(|e|AudioError::Config(e.to_string()))?;let oc=output.default_output_config().map_err(|e|AudioError::Config(e.to_string()))?;if ic.sample_format()!=cpal::SampleFormat::F32||oc.sample_format()!=cpal::SampleFormat::F32{return Err(AudioError::Config("prototype currently requires f32 devices".into()))}let rb=HeapRb::<f32>::new((oc.sample_rate().0 as usize).max(48000));let(mut prod,mut cons)=rb.split();let metrics_in=self.metrics.clone();let channels=ic.channels() as usize;let input_stream=input.build_input_stream(&ic.config(),move|data:&[f32],_|{let mut peak=0f32;for frame in data.chunks(channels){let mono=frame.iter().sum::<f32>()/channels as f32;peak=peak.max(mono.abs());let _=prod.try_push(mono)}metrics_in.input.store(peak.to_bits(),Ordering::Relaxed)},move|e|eprintln!("input stream: {e}"),None).map_err(|e|AudioError::Stream(e.to_string()))?;
 let params=self.parameters.clone();let metrics_out=self.metrics.clone();let out_channels=oc.channels() as usize;let mut dc_x=0f32;let mut dc_y=0f32;let output_stream=output.build_output_stream(&oc.config(),move|data:&mut[f32],_|{let p=*params.read().unwrap();let gate=10f32.powf(p.gate_threshold_db/20.);let mut peak=0f32;for frame in data.chunks_mut(out_channels){let dry=cons.try_pop().unwrap_or_else(||{metrics_out.underruns.fetch_add(1,Ordering::Relaxed);0.});let hp=dry-dc_x+0.995*dc_y;dc_x=dry;dc_y=hp;let gated=if hp.abs()<gate{0.}else{hp};let character=(gated*(2f32.powf(p.pitch_semitones/12.))).tanh();let wet=dry*(1.-p.mix)+character*p.mix;let sample=(wet*p.input_gain*p.output_gain).tanh();peak=peak.max(sample.abs());for v in frame{*v=sample}}metrics_out.output.store(peak.to_bits(),Ordering::Relaxed)},move|e|eprintln!("output stream: {e}"),None).map_err(|e|AudioError::Stream(e.to_string()))?;input_stream.play().map_err(|e|AudioError::Stream(e.to_string()))?;output_stream.play().map_err(|e|AudioError::Stream(e.to_string()))?;self.input=Some(input_stream);self.output=Some(output_stream);self.metrics.running.store(true,Ordering::Release);Ok(())}
 pub fn stop(&mut self){self.input.take();self.output.take();self.metrics.running.store(false,Ordering::Release)}
 pub fn set_parameters(&mut self,p:Parameters){*self.parameters.write().unwrap()=p}
 pub fn status(&self)->EngineStatus{let running=self.metrics.running.load(Ordering::Acquire);EngineStatus{running,input_level:f32::from_bits(self.metrics.input.load(Ordering::Relaxed)),output_level:f32::from_bits(self.metrics.output.load(Ordering::Relaxed)),underruns:self.metrics.underruns.load(Ordering::Relaxed),message:if running{"Processing live audio"}else{"Ready"}.into()}}
}

#[cfg(test)] mod tests{use super::*;#[test]fn defaults_are_valid(){Parameters::default().validate().unwrap()}#[test]fn rejects_bad_mix(){let mut p=Parameters::default();p.mix=2.;assert!(p.validate().is_err())}}
