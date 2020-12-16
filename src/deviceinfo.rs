use std;
use std::io::Read;
use std::{fmt, fs, io, mem, slice};
//use std::vec::Vec;
use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

//use super::usbtypes::*;
use super::*;

const SYSFS_DEVICE_PATH: &'static str = "/sys/bus/usb/devices";

/// Provides metadata about a specific USB device.
///
/// All information is collected from the linux `sysfs` directory.
/// See the function deviceinfo_collection()
#[derive(Debug)]
pub struct DeviceInfo {
    dir: OsString,
}

impl DeviceInfo {

    /// New struct from sysfs device path. Ex
    ///
    /// let di = DeviceInfo::from_devpath("/sys/bus/usb/devices/1-5")?;
    pub fn from_devpath<P: AsRef<OsStr>>(p: P) -> io::Result<DeviceInfo> {
        match is_device_dirname(p.as_ref()) {
            true => Ok(DeviceInfo{dir: p.as_ref().to_os_string()}),
            false => Err(io::Error::new(io::ErrorKind::Other, "path fails device test"))
        }
    }

    /// Something about device_descriptor.
    pub fn device_descriptor(&self) -> io::Result<DeviceDescriptor<NativeEndian>> {
        let mut descr: DeviceDescriptor<BusEndian> =
            unsafe { mem::MaybeUninit::uninit().assume_init() };
        let filename = fmt::format(format_args!(
            "{}/{}/descriptors",
            SYSFS_DEVICE_PATH,
            self.dir.to_str().unwrap()
        ));
        let buf: &mut [u8] = unsafe {
            slice::from_raw_parts_mut(
                &mut descr as *mut DeviceDescriptor<BusEndian> as *mut u8,
                mem::size_of::<DeviceDescriptor<BusEndian>>(),
            )
        };
        fs::File::open(filename)?.read_exact(buf)?;
        Ok(descr.into())
    }
    pub fn busnum(&self) -> io::Result<u32> {
        read_sysfs_num(self.dir.to_str().unwrap(), "busnum")
    }
    pub fn devnum(&self) -> io::Result<u32> {
        read_sysfs_num(self.dir.to_str().unwrap(), "devnum")
    }
}

fn read_sysfs_num<T: std::str::FromStr>(dirname: &str, attr: &str) -> io::Result<T> {
    let filename = fmt::format(format_args!("{}/{}/{}", SYSFS_DEVICE_PATH, dirname, attr));
    let mut buf = String::new();
    fs::File::open(filename)?.read_to_string(&mut buf).unwrap();
    buf.trim()
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "bad parse"))
}

// Someday return just an iterator instead of a collection.
// Rust needs to support return type inference first.
pub fn deviceinfo_enumerate() -> impl Iterator<Item = DeviceInfo> {
    fs::read_dir(SYSFS_DEVICE_PATH)
        .into_iter()
        .flat_map(|x| x) // produce empty iterator if read_dir failed
        .filter_map(|x| x.ok()) // discard erroneous dir entries
        .map(|x| x.file_name())
        .filter(|x| is_device_dirname(x)) //discard non-device filnames
        .map(|x| DeviceInfo { dir: x })
}

/// Provide collection of `DeviceInfo` instances representing
/// all USB devices on the host.
///
/// # Examples
///
/// Show Device Descriptors for all USB devices:
///
/// ```
/// use usbfs::*;
/// for di in deviceinfo_enumerate() {
///     let desc = di.device_descriptor().unwrap();
///     println!("device descriptor = {:?}", desc);
/// }
/// ```
///
/// Find a specific device:
///
/// ```
/// use usbfs::*
/// fn main() {}
///     let mydev_info = deviceinfo_enumerate().find(is_my_device).unwrap()
///     // ...
/// }
///
/// // find my custom LPCxpresso device
/// fn is_my_device(di: &DeviceInfo) -> bool {
///     match di.device_descriptor() {
///         Ok(descr) => (descr.idVendor == 0xffff) && (descr.idProduct == 3),
///         _ => false
///     }
/// }
/// ```
///
fn is_device_dirname<P: AsRef<Path>>(dirname: P) -> bool {
    dirname
        .as_ref() // &Path
        .file_name() // Option<&OsStr>
        .map(|x| x.as_bytes()) // Option<&[u8]>
        .map(|x| !x.starts_with(b"usb") && !x.contains(&b':'))
        .unwrap_or(false)
}
