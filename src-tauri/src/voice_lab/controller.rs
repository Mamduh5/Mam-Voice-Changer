use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
        Arc,
    },
    thread,
};

use crate::dsp::chain::DspParameters;

use super::session::{ClipVersion, VoiceLabSession, VoiceLabStatus};

type StatusReply = SyncSender<Result<VoiceLabStatus, String>>;

enum VoiceLabCommand {
    Status {
        reply: StatusReply,
    },
    StartCapture {
        input_id: String,
        input_name: String,
        reply: StatusReply,
    },
    StopCapture {
        reply: StatusReply,
    },
    ImportWav {
        path: PathBuf,
        reply: StatusReply,
    },
    Render {
        parameters: DspParameters,
        reply: StatusReply,
    },
    StartPreview {
        version: ClipVersion,
        output_id: String,
        output_name: String,
        looping: bool,
        reply: StatusReply,
    },
    StopPreview {
        reply: StatusReply,
    },
    StopAudio {
        reply: StatusReply,
    },
    ExportWav {
        version: ClipVersion,
        path: PathBuf,
        reply: SyncSender<Result<(), String>>,
    },
    Clear {
        reply: StatusReply,
    },
}

pub struct VoiceLabController {
    commands: SyncSender<VoiceLabCommand>,
    audio_active: Arc<AtomicBool>,
}

impl VoiceLabController {
    pub fn new() -> Result<Self, String> {
        let (commands, receiver) = mpsc::sync_channel(16);
        let audio_active = Arc::new(AtomicBool::new(false));
        let thread_audio_active = Arc::clone(&audio_active);
        thread::Builder::new()
            .name("voice-lab-session".to_owned())
            .spawn(move || run_session(receiver, thread_audio_active))
            .map_err(|error| format!("Cannot start the Voice Lab session: {error}"))?;
        Ok(Self {
            commands,
            audio_active,
        })
    }

    pub fn is_audio_active(&self) -> bool {
        self.audio_active.load(Ordering::Acquire)
    }

    pub fn status(&self) -> Result<VoiceLabStatus, String> {
        self.request_status(|reply| VoiceLabCommand::Status { reply })
    }

    pub fn start_capture(
        &self,
        input_id: String,
        input_name: String,
    ) -> Result<VoiceLabStatus, String> {
        self.request_status(|reply| VoiceLabCommand::StartCapture {
            input_id,
            input_name,
            reply,
        })
    }

    pub fn stop_capture(&self) -> Result<VoiceLabStatus, String> {
        self.request_status(|reply| VoiceLabCommand::StopCapture { reply })
    }

    pub fn import_wav(&self, path: PathBuf) -> Result<VoiceLabStatus, String> {
        self.request_status(|reply| VoiceLabCommand::ImportWav { path, reply })
    }

    pub fn render(&self, parameters: DspParameters) -> Result<VoiceLabStatus, String> {
        self.request_status(|reply| VoiceLabCommand::Render { parameters, reply })
    }

    pub fn start_preview(
        &self,
        version: ClipVersion,
        output_id: String,
        output_name: String,
        looping: bool,
    ) -> Result<VoiceLabStatus, String> {
        self.request_status(|reply| VoiceLabCommand::StartPreview {
            version,
            output_id,
            output_name,
            looping,
            reply,
        })
    }

    pub fn stop_preview(&self) -> Result<VoiceLabStatus, String> {
        self.request_status(|reply| VoiceLabCommand::StopPreview { reply })
    }

    pub fn stop_audio(&self) -> Result<VoiceLabStatus, String> {
        self.request_status(|reply| VoiceLabCommand::StopAudio { reply })
    }

    pub fn export_wav(&self, version: ClipVersion, path: PathBuf) -> Result<(), String> {
        let (reply, response) = mpsc::sync_channel(1);
        self.commands
            .send(VoiceLabCommand::ExportWav {
                version,
                path,
                reply,
            })
            .map_err(|_| "Voice Lab is unavailable.".to_owned())?;
        response
            .recv()
            .map_err(|_| "Voice Lab stopped before export completed.".to_owned())?
    }

    pub fn clear(&self) -> Result<VoiceLabStatus, String> {
        self.request_status(|reply| VoiceLabCommand::Clear { reply })
    }

    fn request_status(
        &self,
        command: impl FnOnce(StatusReply) -> VoiceLabCommand,
    ) -> Result<VoiceLabStatus, String> {
        let (reply, response) = mpsc::sync_channel(1);
        self.commands
            .send(command(reply))
            .map_err(|_| "Voice Lab is unavailable.".to_owned())?;
        response
            .recv()
            .map_err(|_| "Voice Lab stopped before the operation completed.".to_owned())?
    }
}

fn run_session(receiver: Receiver<VoiceLabCommand>, audio_active: Arc<AtomicBool>) {
    let mut session = VoiceLabSession::default();
    while let Ok(command) = receiver.recv() {
        match command {
            VoiceLabCommand::Status { reply } => {
                reply_status(&mut session, reply, Ok(()), &audio_active)
            }
            VoiceLabCommand::StartCapture {
                input_id,
                input_name,
                reply,
            } => {
                let result = session.start_capture(&input_id, &input_name);
                reply_status(&mut session, reply, result, &audio_active);
            }
            VoiceLabCommand::StopCapture { reply } => {
                let result = session.stop_capture();
                reply_status(&mut session, reply, result, &audio_active);
            }
            VoiceLabCommand::ImportWav { path, reply } => {
                let result = session.import_wav(&path);
                reply_status(&mut session, reply, result, &audio_active);
            }
            VoiceLabCommand::Render { parameters, reply } => {
                let result = session.render(parameters);
                reply_status(&mut session, reply, result, &audio_active);
            }
            VoiceLabCommand::StartPreview {
                version,
                output_id,
                output_name,
                looping,
                reply,
            } => {
                let result = session.start_preview(version, &output_id, &output_name, looping);
                reply_status(&mut session, reply, result, &audio_active);
            }
            VoiceLabCommand::StopPreview { reply } => {
                session.stop_preview();
                reply_status(&mut session, reply, Ok(()), &audio_active);
            }
            VoiceLabCommand::StopAudio { reply } => {
                let result = session.stop_audio();
                reply_status(&mut session, reply, result, &audio_active);
            }
            VoiceLabCommand::ExportWav {
                version,
                path,
                reply,
            } => {
                let _ = reply.send(session.export_wav(version, &path));
            }
            VoiceLabCommand::Clear { reply } => {
                let result = session.clear();
                reply_status(&mut session, reply, result, &audio_active);
            }
        }
    }
    let _ = session.stop_audio();
    audio_active.store(false, Ordering::Release);
}

fn reply_status(
    session: &mut VoiceLabSession,
    reply: StatusReply,
    result: Result<(), String>,
    audio_active: &AtomicBool,
) {
    let response = result.map(|()| session.status());
    audio_active.store(session.is_audio_active(), Ordering::Release);
    let _ = reply.send(response);
}
