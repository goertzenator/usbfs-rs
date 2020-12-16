
use::std::{marker};


/// Marker type for dual-endian structs.
///
/// Indicates endian used in USB (little endian.)
#[derive(Debug, Copy, Clone)]
pub struct BusEndian;


/// Marker type for dual-endian structs.
///
/// Indicates endian used on the host (big or little endian depending on platform.)
#[derive(Debug, Copy, Clone)]
pub struct NativeEndian;


// Maybe something better can be done with endianness and serde, such as:
// Treat BusEndian as byte array and have C struct endian decoder/encoder.


/// // usb_types

/// Control request direction, part of Setup::bmRequestType.
#[derive(Debug, Copy, Clone)]
pub enum SetupDirection {
    HostToDevice = 0,
    DeviceToHost = 1<<7,
}

/// Control request type, part of Setup::bmRequestType.
#[derive(Debug, Copy, Clone)]
pub enum SetupType {
    Standard = 0<<5,
    Class = 1<<5,
    Vendor = 2<<5,
}

/// Control request recipient, part of Setup::bmRequestType.
#[derive(Debug, Copy, Clone)]
pub enum SetupRecipient {
    Device = 0,
    Interface = 1,
    Endpoint = 2,
    Other = 3,
}

/// USB [Setup packet](http://www.beyondlogic.org/usbnutshell/usb6.shtml) used for Control requests.
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct Setup<E> {
    pub bmRequestType: u8,
    pub bRequest: u8,
    pub wValue: u16,
    pub wIndex: u16,
    pub wLength: u16,
    endian: marker::PhantomData<E>,
}

impl Setup<NativeEndian> {
    /// Construct a native-endian Setup packet.
    ///
    /// `setupdirection`, `setuptype`, and `setuprecipient` are combined to form `bmRequestType`.
    pub fn new(setupdirection: SetupDirection,
               setuptype: SetupType,
               setuprecipient: SetupRecipient,
               request: u8,
               value: u16,
               index: u16,
               length: u16)
               -> Setup<NativeEndian> {
        Setup {
            bmRequestType: (setupdirection as u8) | (setuptype as u8) | (setuprecipient as u8),
            bRequest: request,
            wValue: value,
            wIndex: index,
            wLength: length,
            endian: marker::PhantomData,
        }
    }
}

impl From<Setup<NativeEndian>> for Setup<BusEndian> {
    fn from(f: Setup<NativeEndian>) -> Setup<BusEndian> {
        Setup {
            bmRequestType: f.bmRequestType.to_le(),
            bRequest: f.bRequest.to_le(),
            wValue: f.wValue.to_le(),
            wIndex: f.wIndex.to_le(),
            wLength: f.wLength.to_le(),
            endian: marker::PhantomData,
        }
    }
}


/// USB [Device Descriptor](http://www.beyondlogic.org/usbnutshell/usb5.shtml)
/// used for examining USB devices attached to the host.
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct DeviceDescriptor<E> {
    pub bLength: u8,
    pub bDescriptorType: u8,
    pub bcdUSB: u16,
    pub bDeviceClass: u8,
    pub bDeviceSubClass: u8,
    pub bDeviceProtocol: u8,
    pub bMaxPacketSize0: u8,
    pub idVendor: u16,
    pub idProduct: u16,
    pub bcdDevice: u16,
    pub iManufacturer: u8,
    pub iProduct: u8,
    pub iSerialNumber: u8,
    pub bNumConfigurations: u8,
    endian: marker::PhantomData<E>,
}

impl From<DeviceDescriptor<BusEndian>> for DeviceDescriptor<NativeEndian> {
    fn from(f: DeviceDescriptor<BusEndian>) -> DeviceDescriptor<NativeEndian> {
        DeviceDescriptor {
            bLength: u8::from_le(f.bLength),
            bDescriptorType: u8::from_le(f.bDescriptorType),
            bcdUSB: u16::from_le(f.bcdUSB),
            bDeviceClass: u8::from_le(f.bDeviceClass),
            bDeviceSubClass: u8::from_le(f.bDeviceSubClass),
            bDeviceProtocol: u8::from_le(f.bDeviceProtocol),
            bMaxPacketSize0: u8::from_le(f.bMaxPacketSize0),
            idVendor: u16::from_le(f.idVendor),
            idProduct: u16::from_le(f.idProduct),
            bcdDevice: u16::from_le(f.bcdDevice),
            iManufacturer: u8::from_le(f.iManufacturer),
            iProduct: u8::from_le(f.iProduct),
            iSerialNumber: u8::from_le(f.iSerialNumber),
            bNumConfigurations: u8::from_le(f.bNumConfigurations),
            endian: marker::PhantomData,
        }
    }
}

// Definitions corresponding to https://github.com/torvalds/linux/blob/master/include/uapi/linux/usbdevice_fs.h

