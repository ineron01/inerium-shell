use crate::scrcpy::PREVIEW_SHM_SOCKET;
use gdk4::prelude::*;
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use gtk4::Picture;
use std::path::Path;

pub struct LivePreviewPipeline {
    pipeline: Option<gst::Pipeline>,
}

impl LivePreviewPipeline {
    pub fn new() -> Self {
        let _ = gst::init();
        Self { pipeline: None }
    }

    /// Check whether preview pipeline is already attached to SHM socket
    pub fn is_running(&self) -> bool {
        self.pipeline.is_some()
    }

    /// Stop any running preview pipeline
    pub fn stop(&mut self) {
        if let Some(pipe) = self.pipeline.take() {
            let _ = pipe.set_state(gst::State::Null);
        }
    }

    /// Start preview pipeline reading unified 1920x1080 SHM stream (zero conflict with /dev/video0 readers)
    pub fn start(&mut self, picture: Picture) -> Result<(), String> {
        self.stop();

        if !Path::new(PREVIEW_SHM_SOCKET).exists() {
            return Err("Видеопоток превью еще не инициализирован".to_string());
        }

        let pipe_str = format!(
            "shmsrc socket-path=\"{}\" is-live=true do-timestamp=true ! \
             video/x-raw,format=I420,width=1920,height=1080,framerate=30/1 ! \
             videoconvert ! video/x-raw,format=RGBA ! \
             appsink name=preview_sink drop=true max-buffers=1 sync=false",
            PREVIEW_SHM_SOCKET
        );

        let element = gst::parse::launch(&pipe_str)
            .map_err(|e| format!("Ошибка создания GStreamer превью: {}", e))?;

        let pipeline = element
            .dynamic_cast::<gst::Pipeline>()
            .map_err(|_| "Не удалось привести элемент к Pipeline".to_string())?;

        let sink = pipeline
            .by_name("preview_sink")
            .ok_or_else(|| "Элемент preview_sink не найден".to_string())?
            .dynamic_cast::<gst_app::AppSink>()
            .map_err(|_| "Не удалось привести preview_sink к AppSink".to_string())?;

        // Bounded channel drops stale frames; task terminates automatically when pipeline drops
        let (tx, mut rx) = tokio::sync::mpsc::channel::<(i32, i32, Vec<u8>)>(1);

        let pic_clone = picture.clone();
        glib::spawn_future_local(async move {
            while let Some((w, h, data)) = rx.recv().await {
                let bytes = glib::Bytes::from_owned(data);
                let texture = gdk4::MemoryTexture::new(
                    w,
                    h,
                    gdk4::MemoryFormat::R8g8b8a8,
                    &bytes,
                    (w * 4) as usize,
                );
                pic_clone.set_paintable(Some(&texture));
            }
        });

        sink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |appsink| {
                    let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Error)?;
                    let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;
                    let map = buffer.map_readable().map_err(|_| gst::FlowError::Error)?;

                    let (w, h) = sample
                        .caps()
                        .and_then(|caps| caps.structure(0))
                        .map(|s| {
                            (
                                s.get::<i32>("width").unwrap_or(1920),
                                s.get::<i32>("height").unwrap_or(1080),
                            )
                        })
                        .unwrap_or((1920, 1080));

                    let data = map.as_slice().to_vec();
                    let _ = tx.try_send((w, h, data));

                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );

        pipeline
            .set_state(gst::State::Playing)
            .map_err(|e| format!("Ошибка запуска воспроизведения превью: {:?}", e))?;

        self.pipeline = Some(pipeline);
        Ok(())
    }
}

impl Default for LivePreviewPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for LivePreviewPipeline {
    fn drop(&mut self) {
        self.stop();
    }
}
