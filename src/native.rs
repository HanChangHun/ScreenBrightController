use crate::controller::{monitor, Driver, Monitor, DISCONNECTED};
use crate::Ramp;
use std::ptr;
use windows_sys::Win32::{
    Graphics::Gdi::{
        CreateDCW, DeleteDC, EnumDisplayDevicesW, DISPLAY_DEVICEW,
        DISPLAY_DEVICE_ATTACHED_TO_DESKTOP, DISPLAY_DEVICE_MIRRORING_DRIVER, HDC,
    },
    UI::ColorSystem::{GetDeviceGammaRamp, SetDeviceGammaRamp},
};
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
fn decode(s: &[u16]) -> String {
    String::from_utf16_lossy(&s[..s.iter().position(|v| *v == 0).unwrap_or(s.len())])
}
struct Dc(HDC);
impl Drop for Dc {
    fn drop(&mut self) {
        unsafe {
            DeleteDC(self.0);
        }
    }
}
fn open(id: &str) -> Result<Dc, String> {
    if !id.starts_with("\\\\.\\DISPLAY") || id.contains('\0') {
        return Err("invalid display identifier".into());
    }
    let driver = wide("DISPLAY");
    let device = wide(id);
    let dc = unsafe { CreateDCW(driver.as_ptr(), device.as_ptr(), ptr::null(), ptr::null()) };
    if dc.is_null() {
        Err(format!(
            "CreateDCW failed for {id}: {}",
            std::io::Error::last_os_error()
        ))
    } else {
        Ok(Dc(dc))
    }
}
pub struct Native;
impl Driver for Native {
    fn snapshot(&mut self) -> Result<Vec<Monitor>, String> {
        let mut out = vec![];
        for index in 0..128 {
            let mut d = DISPLAY_DEVICEW {
                cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
                ..Default::default()
            };
            if unsafe { EnumDisplayDevicesW(ptr::null(), index, &mut d, 0) } == 0 {
                break;
            }
            if d.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP == 0
                || d.StateFlags & DISPLAY_DEVICE_MIRRORING_DRIVER != 0
            {
                continue;
            }
            let id = decode(&d.DeviceName);
            let read = self.read(&id);
            out.push(Monitor {
                id,
                name: decode(&d.DeviceString),
                original: read.as_ref().ok().cloned(),
                error: read.err(),
            });
        }
        Ok(out)
    }
    fn arm_continuous(&mut self, _id: &str) -> Result<(), String> {
        Err("native backend requires independent watchdog".into())
    }
    fn read(&mut self, id: &str) -> Result<Ramp, String> {
        let dc = open(id)?;
        let mut ramp = [[0u16; 256]; 3];
        if unsafe { GetDeviceGammaRamp(dc.0, ramp.as_mut_ptr().cast()) } == 0 {
            return Err(format!(
                "GetDeviceGammaRamp failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(ramp.iter().map(|c| c.to_vec()).collect())
    }
    fn set(&mut self, id: &str, ramp: &Ramp) -> Result<bool, String> {
        crate::scale(ramp, 100)?;
        let dc = open(id)?;
        let mut raw = [[0u16; 256]; 3];
        for (dst, src) in raw.iter_mut().zip(ramp) {
            dst.copy_from_slice(src);
        }
        Ok(unsafe { SetDeviceGammaRamp(dc.0, raw.as_ptr().cast()) } != 0)
    }
    fn restore(&mut self, id: &str, original: &Ramp) -> Result<(), String> {
        // Skip disconnected IDs. Windows may recycle display IDs after hotplug;
        // hotplug/replacement while dimmed is unsupported (see README).
        if monitor(&self.snapshot()?, id).is_none() {
            return Err(DISCONNECTED.into());
        }
        if !self.set(id, original)? {
            return Err("restore API returned false".into());
        }
        if self.read(id)? != *original {
            return Err(
                "restore API succeeded but readback differs; visible restoration unverified".into(),
            );
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn readonly_native_enumeration_returns_well_formed_snapshots() {
        let monitors = Native.snapshot().unwrap();
        assert!(!monitors.is_empty());
        for m in monitors {
            assert!(m.id.starts_with("\\\\.\\DISPLAY"));
            if let Some(r) = m.original {
                assert_eq!(r.len(), 3);
                assert!(r.iter().all(|c| c.len() == 256));
            } else {
                assert!(m.error.is_some());
            }
        }
    }
}
