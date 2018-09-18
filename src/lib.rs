extern crate winreg;
extern crate libc;
#[macro_use]
extern crate bitflags;

use std::ffi;
use std::io;
use std::fmt;
use std::error;
use std::str::Utf8Error;
use winreg::RegKey;
use winreg::enums::*;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct Error {
    kind: ErrorKind,
}

#[derive(Copy, Clone, Debug)]
pub enum ErrorKind {
    NotFound,
    Code(i32),
    Utf8,
}

impl Error {
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    pub fn from_code(code: i32) -> Error {
        Error { kind: ErrorKind::Code(code) }
    }

    fn as_str(&self) -> &str {
        match self.kind {
            ErrorKind::NotFound => "not found",
            ErrorKind::Code(code) => match code {
                _ => "unknown error",
            },
            ErrorKind::Utf8 => "utf8 error",
        }
    }
}

impl From<Utf8Error> for Error {
    fn from(err: Utf8Error) -> Self {
        Error {
            kind: ErrorKind::Utf8,
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        self.as_str()
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

extern {
    fn j2534_load(path: *const libc::c_char) -> *mut libc::c_void;
    fn j2534_close(handle: *const libc::c_void);
    fn j2534_PassThruClose(handle: *const libc::c_void, device_id: libc::uint32_t) -> libc::int32_t;
    fn j2534_PassThruOpen(handle: *const libc::c_void, port: *const libc::c_char, device_id: *mut libc::uint32_t) -> libc::int32_t;
    fn j2534_PassThruConnect(handle: *const libc::c_void, device_id: libc::uint32_t, protocol_id: libc::uint32_t, flags: libc::uint32_t, baudrate: libc::uint32_t, channel_id: *mut libc::uint32_t) -> libc::int32_t;
    fn j2534_PassThruDisconnect(handle: *const libc::c_void, channel_id: libc::uint32_t) -> libc::int32_t;
    fn j2534_PassThruReadMsgs(handle: *const libc::c_void, channel_id: libc::uint32_t, num_msgs: *mut libc::uint32_t, timeout: libc::uint32_t) -> libc::int32_t;
    fn j2534_PassThruWriteMsgs(handle: *const libc::c_void, channel_id: libc::uint32_t, num_msgs: *mut libc::uint32_t, timeout: libc::uint32_t) -> libc::int32_t;
    fn j2534_PassThruStartPeriodicMsg(handle: *const libc::c_void, channel_id: libc::uint32_t, msg: *mut PassthruMsg, msg_id: *mut libc::uint32_t, time_interval: libc::uint32_t) -> libc::int32_t;
    fn j2534_PassThruStopPeriodicMsg(handle: *const libc::c_void, channel_id: libc::uint32_t, msg_id: libc::uint32_t) -> libc::int32_t;
    fn j2534_PassThruStartMsgFilter(handle: *const libc::c_void, channel_id: libc::uint32_t, filter_type: libc::uint32_t, msg_mask: *mut PassthruMsg, pattern_msg: *mut PassthruMsg, flow_control_msg: *mut PassthruMsg, filter_id: *mut libc::uint32_t) -> libc::int32_t;
    fn j2534_PassThruStopMsgFilter(handle: *const libc::c_void, channel_id: libc::uint32_t, filter_id: libc::uint32_t) -> libc::int32_t;
    fn j2534_PassThruSetProgrammingVoltage(handle: *const libc::c_void, device_id: libc::uint32_t, pin_number: libc::uint32_t, voltage: libc::uint32_t) -> libc::int32_t;
    fn j2534_PassThruReadVersion(handle: *const libc::c_void, device_id: libc::uint32_t, firmware_version: *mut libc::c_char, dll_version: *mut libc::c_char, api_version: *mut libc::c_char) -> libc::int32_t;
    fn j2534_PassThruGetLastError(handle: *const libc::c_void, error_description: *mut libc::c_char) -> libc::int32_t;
    fn j2534_PassThruIoctl(handle: *const libc::c_void, handle_id: libc::uint32_t, ioctl_id: libc::uint32_t, input: *mut libc::c_void, output: *mut libc::c_void) -> libc::int32_t;
}

#[repr(C)]
pub struct PassthruMsg {
    pub protocol_id: u32,
    pub rx_status: u32,
    pub tx_flags: u32,
    pub timestamp: u32,
    pub data_size: u32,
    pub extra_data_index: u32,
    pub data: [u8; 4128],
}

/// Represents a J2534 library
pub struct Interface {
    handle: *const libc::c_void,
}

/// Represents a J2534 device created with `Interface::open`
pub struct Device<'a> {
    interface: &'a Interface,
    id: u32,
}

/// Represents a J2534 channel
pub struct Channel<'a> {
    device: &'a Device<'a>,
    id: u32,
}

impl Interface {
    /// Returns a J2534 library given the path
    /// 
    /// # Arguments
    /// 
    /// * `path` - The absolute path to the J2534 shared library
    /// 
    /// # Example
    /// ```
    /// use j2534::Interface;
    /// let interface = Interface::new("C:\\j2534_driver.dll").unwrap();
    /// ```
    pub fn new(path: &str) -> Result<Interface> {
        let cstring  = ffi::CString::new(path).unwrap();
        let handle = unsafe { j2534_load(cstring.as_ptr()) };
        if handle.is_null() {
            return Err(Error{kind: ErrorKind::NotFound});
        }
        Ok(Interface{handle})
    }

    /// Creates a `Device` from the J2534 connected to the given port
    /// 
    /// # Arguments
    /// 
    /// * `port` - The port to search for a J2534 device
    /// 
    /// # Example
    /// ```
    /// use j2534::Interface;
    /// let interface = Interface::new("C:\\j2534_driver.dll").unwrap();
    /// let device = interface.open("COM2").unwrap();
    /// ```
    pub fn open(&self, port: &str) -> Result<Device> {
        let s = ffi::CString::new(port).unwrap();
        let raw = s.as_ptr();
        let mut id = 0;
        
        let res = unsafe { j2534_PassThruOpen(self.handle, raw, &mut id as *mut libc::uint32_t) };
        if res != 0 {
            return Err(Error::from_code(res));
        }

        Ok(Device {interface: self, id})
    }

    /// Creates a `Device` from any connected J2534 devices
    /// 
    /// # Example
    /// ```
    /// use j2534::Interface;
    /// let interface = Interface::new("C:\\j2534_driver.dll").unwrap();
    /// let device = interface.open_any().unwrap();
    /// ```
    pub fn open_any(&self) -> Result<Device> {
        let raw = 0 as *const libc::c_void;
        let mut id = 0;
        let res = unsafe { j2534_PassThruOpen(self.handle, raw as *const libc::c_char, &mut id as *mut libc::uint32_t) };
        if res != 0 {
            return Err(Error::from_code(res));
        }

        Ok(Device {interface: self, id})
    }
}

impl Drop for Interface {
    fn drop(&mut self) {
        unsafe { j2534_close(self.handle) };
    }
}

pub enum Protocol {
    J1850VPW = 1,
    J1850PWM = 2,
    ISO9141 = 3,
    ISO14230 = 4,
    CAN = 5,
    ISO15765 = 6,
    SCI_A_ENGINE = 7,
    SCI_A_TRANS = 8,
    SCI_B_ENGINE = 9,
    SCI_B_TRANS = 10,
}

bitflags! {
    pub struct ConnectFlags: u32 {
        const NONE = 0;
        const CAN_29_BIT_ID = 0x100;
        const ISO9141_NO_CHECKSUM = 0x200;
        const CAN_ID_BOTH = 0x800;
        const ISO9141_K_LINE_ONLY = 0x1000;
    }
}

#[derive(Debug)]
pub struct VersionInfo {
    pub firmware_version: String,
    pub dll_version: String,
    pub api_version: String,
}

impl<'a> Device<'a> {
    pub fn connect_raw(&self, protocol: u32, flags: u32, baudrate: u32) -> Result<Channel> {
        let mut id: u32 = 0;
        let res = unsafe { j2534_PassThruConnect(self.interface.handle, self.id, protocol, flags, baudrate, &mut id as *mut libc::uint32_t) };
        if res != 0 {
            return Err(Error::from_code(res));
        }
        Ok(Channel {
            device: self,
            id
        })
    }

    pub fn connect(&self, protocol: Protocol, flags: ConnectFlags, baudrate: u32) -> Result<Channel> {
        self.connect_raw(protocol as u32, flags.bits(), baudrate)
    }

    pub fn read_version(&self) -> Result<VersionInfo> {
        let mut firmware_version: [u8; 80] = [0; 80];
        let mut dll_version: [u8; 80] = [0; 80];
        let mut api_version: [u8; 80] = [0; 80];
        let res = unsafe { j2534_PassThruReadVersion(self.interface.handle, self.id, firmware_version.as_mut_ptr() as *mut libc::c_char, dll_version.as_mut_ptr() as *mut libc::c_char, api_version.as_mut_ptr() as *mut libc::c_char) };
        if res != 0 {
            return Err(Error::from_code(res));
        }
        unsafe {
            Ok(VersionInfo {
                firmware_version: String::from(ffi::CStr::from_ptr(firmware_version.as_mut_ptr() as *mut libc::c_char).to_str()?),
                api_version: String::from(ffi::CStr::from_ptr(api_version.as_mut_ptr() as *mut libc::c_char).to_str()?),
                dll_version: String::from(ffi::CStr::from_ptr(dll_version.as_mut_ptr() as *mut libc::c_char).to_str()?),
            })
        }
    }
}

impl<'a> Drop for Device<'a> {
    fn drop(&mut self) {
        unsafe { j2534_PassThruClose(self.interface.handle, self.id) };
    }
}

impl<'a> Channel<'a> {
    
}

impl<'a> Drop for Channel<'a> {
    fn drop(&mut self) {
        unsafe { j2534_PassThruDisconnect(self.device.interface.handle, self.id) };
    }
}


#[derive(Debug)]
pub struct Listing {
    pub name: String,
    pub vendor: String,
    pub path: String,
}

/// Returns a list of all installed PassThru drivers
pub fn list() -> io::Result<Vec<Listing>> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    let software = hklm.open_subkey("SOFTWARE")?;
    let passthru = software.open_subkey("PassThruSupport.04.04");
    if let Err(err) = passthru {
        if err.kind() == io::ErrorKind::NotFound {
            return Ok(Vec::new());
        }
        return Err(err);
    }
    let passthru = passthru.unwrap();
    let mut listings = Vec::new();

    for name in passthru.enum_keys() {
        let name = name?;
        let key = passthru.open_subkey(name)?;

        let device_name: String = key.get_value("Name")?;
        let vendor: String = key.get_value("Vendor")?;
        let path: String = key.get_value("FunctionLibrary")?;

        listings.push(Listing {name: device_name, vendor: vendor, path: path});
    }

    Ok(listings)
}