extern crate usbfs;
//extern crate mio;

use usbfs::*;
//use mio::*;
use std::io;


/// This demo application performs a control transfer with a custom USB device using
/// the various techniques supported by the usbfs crate.

fn main() {
    sync_demo().unwrap();
//    sync_transfer_demo().unwrap();
//    async_transfer_poll_demo().unwrap();
//    async_transfer_mio_demo().unwrap();
//    discard_demo().unwrap();
}


/// Perform a synchronous transfer using USBFS blocking call.
fn sync_demo() -> io::Result<()> {
    println!("sync_demo()");


//    for x in deviceinfo_enumerate() {
//        println!("{:?}", x);
//        println!("{:?}", x.device_descriptor());
//        println!("{:?}", x.busnum());
//        println!("{:?}", x.devnum());
//    }

    let device = Device::new(&get_my_device()).unwrap();
    let mut unique_id:[u8;16] = [0x5a;16];

    device.control_transfer( SetupDirection::DeviceToHost,
                             SetupType::Vendor,
                             SetupRecipient::Interface,
                             3, // request (gets HW serial number)
                             0, // value (ignored for this request)
                             0, // index (Watchdog)
                             Some(&mut unique_id),
                             1000).unwrap();
    print!("HW serial = ");
    printbuf(&unique_id);

    // select full DDC112 interface
    device.set_interface(2,1).unwrap();


    // start streaming
    device.control_transfer( SetupDirection::HostToDevice,
                             SetupType::Vendor,
                             SetupRecipient::Interface,
                             0, // request (STREAM_ON)
                             0, // value (ignored for this request)
                             2, // index (DDC112)
                             Some(&mut unique_id),
                             1000).unwrap();

    // stop streaming
    device.control_transfer( SetupDirection::HostToDevice,
                             SetupType::Vendor,
                             SetupRecipient::Interface,
                             1, // request (STREAM_OFF)
                             0, // value (ignored for this request)
                             2, // index (DDC112)
                             Some(&mut unique_id),
                             1000).unwrap();



    Ok(())
}
//
///// Perform a synchronous transfer using blocking reap.
//fn sync_transfer_demo() -> io::Result<()> {
//    println!("async_transfer_demo()");
//
//    let mut device = Device::new(&get_my_device()).unwrap();
//    device.submit(Box::new(make_transfer())).unwrap();
//    match device.reap_wait() {
//        Ok(xfer) => {
//            match xfer.result_data() {
//                Ok(buf) => {
//                    print!("HW serial = ");
//                    printbuf(buf);
//                },
//                Err(_) => panic!("transfer had an error!"),
//            }
//        },
//        Err(_err) =>
//            panic!("reap had an error!"),
//    };
//    Ok(())
//}
//
///// Perform an asynchronous transfer using polling and nonblocking reap.
//fn async_transfer_poll_demo() -> io::Result<()> {
//    println!("async_transfer_poll_demo()");
//
//    let mut device = Device::new(&get_my_device()).unwrap();
//    device.submit(Box::new(make_transfer())).unwrap();
//    loop {
//        match device.reap_nowait() {
//            Ok(xfer) => {
//                match xfer.result_data() {
//                    Ok(buf) => {
//                        print!("HW serial = ");
//                        printbuf(buf);
//                    },
//                    Err(_) => panic!("transfer had an error!"),
//                }
//                break;
//            },
//            Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
//                sleep(Duration::from_millis(1));
//                println!("Zzz..");
//                continue;
//            }
//            Err(_err) =>
//                panic!("reap had an error!"),
//        }
//    }
//    Ok(())
//}

/// Perform asynchronous transfers using nonblocking reap and mio.
//fn async_transfer_mio_demo() -> io::Result<()> {
//    println!("async_transfer_mio_demo()");
//
//    // setup mio handler and event loop
//    let mut myhandler = MyHandler{device: Device::new(&get_my_device()).unwrap()};
//    let mut event_loop = EventLoop::new().unwrap();
//    event_loop.register(&myhandler.device, Token(0), EventSet::writable(),
//                    PollOpt::level()).unwrap();
//
//    // start the first transfer
//    myhandler.device.submit(Box::new(make_transfer())).unwrap();
//
//    // run the loop forever
//    //event_loop.run(&mut myhandler).unwrap();
//
//    // run the loop a few times
//    for _ in 0..10 {
//        event_loop.run_once(&mut myhandler, None).unwrap();
//    }
//    Ok(())
//}

//struct MyHandler {
//    device: Device<Buf1Transfer<Vec<u8>>>,
//}

//impl Handler for MyHandler {
//    type Timeout = ();
//    type Message = ();
//
//    fn ready(&mut self, _event_loop: &mut EventLoop<MyHandler>, _token: Token, _: EventSet) {
//        let r = self.device.reap_nowait();
//
//        match r {
//            Ok(xfer) => {
//                match xfer.result_data() {
//                    Ok(buf) => {
//                        print!("HW serial = ");
//                        printbuf(buf);
//                        self.device.submit(xfer).unwrap();
//                    },
//                    Err(_) => panic!("transfer had an error!"),
//                }
//            },
//            Err(_err) =>
//                panic!("reap had an error!"),
//        }
//    }
//}
//

//
//
//fn make_transfer() -> Buf1Transfer<Vec<u8>> {
//    let mut buf = Vec::new();
//    buf.resize(72,0);
//    Buf1Transfer::control(SetupDirection::DeviceToHost, SetupType::Vendor, SetupRecipient::Interface,
//                      0, // request for USB device, gets HW serial number
//                      0, // value (ignored for this request)
//                      0, // index (ignored for this request)
//                      UrbFlags::empty(),
//                      buf)
//}
//
//
///// Demonstrate transfer abort.
//fn discard_demo() -> io::Result<()> {
//    println!("discard_demo()");
//
//    let mut device = Device::new(&get_my_device()).unwrap();
//
//    device.submit(Box::new(make_transfer())).unwrap();
//    let slot = device.submit(Box::new(make_transfer())).unwrap();
//
//    device.discard(slot).unwrap();
//    println!("poof!");
//
//    Ok(())
//}


fn get_my_device() -> DeviceInfo {
    deviceinfo_enumerate().find(is_my_device).unwrap()
}

// find my custom LPCxpresso device
fn is_my_device(di: &DeviceInfo) -> bool {
    match di.device_descriptor() {
        Ok(descr) => (descr.idVendor == 0xffff) && (descr.idProduct == 4),
        _ => false
    }
}


fn printbuf(buf: &[u8]) {
    for &byte in buf.iter() {
        print!("{:02x}", byte);
    }
    println!();
}
