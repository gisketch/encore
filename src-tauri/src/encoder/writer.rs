#[cfg(target_os = "macos")]
mod platform {
    use std::{
        ffi::{c_char, c_void, CString},
        fs,
        path::{Path, PathBuf},
        ptr::NonNull,
    };

    const ERROR_CAPACITY: usize = 256;

    unsafe extern "C" {
        fn encore_writer_create(
            path: *const c_char,
            width: u32,
            height: u32,
            error: *mut c_char,
            error_capacity: usize,
        ) -> *mut c_void;
        fn encore_writer_append(
            handle: *mut c_void,
            pixel_buffer: *mut c_void,
            pts_microseconds: i64,
            error: *mut c_char,
            error_capacity: usize,
        ) -> i32;
        fn encore_writer_finish(
            handle: *mut c_void,
            error: *mut c_char,
            error_capacity: usize,
        ) -> i32;
        fn encore_writer_destroy(handle: *mut c_void);
    }

    pub struct VideoWriter {
        handle: Option<NonNull<c_void>>,
        partial_path: PathBuf,
        final_path: PathBuf,
    }

    // AVAssetWriter is confined to the encoder worker thread.
    unsafe impl Send for VideoWriter {}

    impl VideoWriter {
        pub fn create(final_path: &Path, width: u32, height: u32) -> Result<Self, &'static str> {
            let partial_path = partial_path(final_path);
            let _ = fs::remove_file(&partial_path);
            let path = CString::new(partial_path.to_string_lossy().as_bytes())
                .map_err(|_| "encoder_invalid_path")?;
            let mut detail = [0_i8; ERROR_CAPACITY];
            let handle = unsafe {
                encore_writer_create(
                    path.as_ptr(),
                    width,
                    height,
                    detail.as_mut_ptr(),
                    detail.len(),
                )
            };
            let handle = NonNull::new(handle).ok_or("hardware_encoder_unavailable")?;
            Ok(Self {
                handle: Some(handle),
                partial_path,
                final_path: final_path.to_owned(),
            })
        }

        pub fn append(
            &mut self,
            pixel_buffer: *mut c_void,
            pts_us: i64,
        ) -> Result<(), &'static str> {
            let Some(handle) = self.handle else {
                return Err("encoder_not_running");
            };
            let mut detail = [0_i8; ERROR_CAPACITY];
            let status = unsafe {
                encore_writer_append(
                    handle.as_ptr(),
                    pixel_buffer,
                    pts_us,
                    detail.as_mut_ptr(),
                    detail.len(),
                )
            };
            match status {
                0 => Ok(()),
                1 => Err("encoder_backpressure"),
                _ => Err("encoder_append_failed"),
            }
        }

        pub fn finish(mut self) -> Result<PathBuf, &'static str> {
            let Some(handle) = self.handle.take() else {
                return Err("encoder_not_running");
            };
            let mut detail = [0_i8; ERROR_CAPACITY];
            let status =
                unsafe { encore_writer_finish(handle.as_ptr(), detail.as_mut_ptr(), detail.len()) };
            unsafe { encore_writer_destroy(handle.as_ptr()) };
            if status != 0 {
                let _ = fs::remove_file(&self.partial_path);
                return Err("encoder_finalize_failed");
            }
            fs::rename(&self.partial_path, &self.final_path)
                .map_err(|_| "segment_publish_failed")?;
            Ok(self.final_path.clone())
        }
    }

    impl Drop for VideoWriter {
        fn drop(&mut self) {
            if let Some(handle) = self.handle.take() {
                unsafe { encore_writer_destroy(handle.as_ptr()) };
            }
            let _ = fs::remove_file(&self.partial_path);
        }
    }

    fn partial_path(final_path: &Path) -> PathBuf {
        let name = final_path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("segment");
        final_path.with_file_name(format!("{name}.partial.mp4"))
    }
}

#[cfg(target_os = "macos")]
pub use platform::VideoWriter;

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::VideoWriter;
    use screencapturekit::CVPixelBuffer;
    use std::{thread, time::Duration};

    #[test]
    #[ignore = "requires macOS hardware H.264 and writes an inspectable MP4"]
    fn writes_hardware_h264_smoke_segment() {
        let output = std::env::temp_dir().join("encore-hardware-smoke.mp4");
        let _ = std::fs::remove_file(&output);
        let mut writer = VideoWriter::create(&output, 320, 180).expect("hardware writer");
        for frame in 0..90 {
            let pixels = CVPixelBuffer::create(320, 180, 0x4247_5241).expect("BGRA buffer");
            loop {
                match writer.append(pixels.as_ptr(), frame * 33_333) {
                    Ok(()) => break,
                    Err("encoder_backpressure") => thread::sleep(Duration::from_millis(2)),
                    Err(code) => panic!("append failed: {code}"),
                }
            }
        }
        let published = writer.finish().expect("finalized MP4");
        assert_eq!(published, output);
        assert!(std::fs::metadata(&published).unwrap().len() > 0);
        println!("{}", published.display());
    }
}
