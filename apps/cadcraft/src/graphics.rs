//! Windows graphics instance configuration, applied before wgpu can initialize any driver.

use eframe::{NativeOptions, egui_wgpu::WgpuSetup, wgpu::Backends};

/// Restrict the wgpu instance to DirectX 12 or Vulkan on Windows.
///
/// eframe's default includes GL on Windows. Creating that backend can crash inside AMD's
/// atio6axx.dll before adapter selection or any Rust error handling runs, and on Intel graphics
/// it times out during Surface::configure (GpuWaitTimeout).
///
/// When `backend_override` (normally from `WGPU_BACKEND`) is unset, we default to DX12 only.
/// If `backend_override` specifies only GL, we warn and fall back to DX12; if multiple backends
/// are specified, GL is stripped out so unstable OpenGL is never initialized on Windows.
pub fn configure(options: &mut NativeOptions, backend_override: Option<Backends>) {
    if let WgpuSetup::CreateNew(create) = &mut options.wgpu_options.wgpu_setup {
        let mut backends = backend_override.unwrap_or(Backends::DX12);
        if backends.contains(Backends::GL) {
            let non_gl = backends - Backends::GL;
            if non_gl.is_empty() {
                log::warn!("OpenGL backend on Windows is unstable in wgpu; falling back to DirectX 12");
                backends = Backends::DX12;
            } else {
                backends = non_gl;
            }
        }
        create.instance_descriptor.backends = backends;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn configured_backends(backend_override: Option<Backends>) -> Backends {
        let mut options = NativeOptions::default();
        configure(&mut options, backend_override);
        let WgpuSetup::CreateNew(create) = options.wgpu_options.wgpu_setup else {
            panic!("default native options must create a wgpu instance");
        };
        create.instance_descriptor.backends
    }

    #[test]
    fn windows_default_initializes_only_dx12() {
        // eframe's own default (PRIMARY | GL) would initialize OpenGL as well.
        let WgpuSetup::CreateNew(eframe_default) = NativeOptions::default().wgpu_options.wgpu_setup else {
            panic!("default native options must create a wgpu instance");
        };
        assert!(eframe_default.instance_descriptor.backends.contains(Backends::GL));
        assert_eq!(configured_backends(None), Backends::DX12);
    }

    #[test]
    fn explicit_backend_override_is_preserved() {
        for backends in [Backends::VULKAN, Backends::DX12, Backends::VULKAN | Backends::DX12, Backends::empty()] {
            assert_eq!(configured_backends(Some(backends)), backends);
        }
    }

    #[test]
    fn windows_gl_override_falls_back_to_dx12() {
        assert_eq!(configured_backends(Some(Backends::GL)), Backends::DX12);
        assert_eq!(configured_backends(Some(Backends::GL | Backends::DX12)), Backends::DX12);
    }
}
