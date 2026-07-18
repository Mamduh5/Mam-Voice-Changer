import { invoke } from '@tauri-apps/api/core';
export type Device={id:string;name:string;is_default:boolean};
export type Parameters={input_gain:number;output_gain:number;gate_threshold_db:number;pitch_semitones:number;mix:number};
export type Status={running:boolean;input_level:number;output_level:number;underruns:number;message:string};
export const defaults:Parameters={input_gain:1,output_gain:1,gate_threshold_db:-50,pitch_semitones:0,mix:1};
export const api={devices:()=>invoke<{inputs:Device[];outputs:Device[]}>('list_audio_devices'),start:(input:string,output:string,parameters:Parameters)=>invoke<void>('start_engine',{inputId:input,outputId:output,parameters}),stop:()=>invoke<void>('stop_engine'),parameters:(parameters:Parameters)=>invoke<void>('set_parameters',{parameters}),status:()=>invoke<Status>('get_engine_status')};
