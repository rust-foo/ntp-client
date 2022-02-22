use std::net::UdpSocket;

fn main() {
    let time = ntp_main("0.0.0.0:0", "pool.ntp.org:123").unwrap();
    println!("Time is {time}");
}

fn ntp_main(bind_address: &str, ntp_server: &str) -> Result<u64, std::io::Error> {
    let socket = UdpSocket::bind(bind_address)?;
    socket.set_write_timeout(Some(std::time::Duration::from_millis(500)))?;
    socket.set_read_timeout(Some(std::time::Duration::from_millis(500)))?;

    let mut transmit: Vec<u8> = vec![0; 48];
    transmit[0] = 0x1b;

    let mut retries = 3;
    while retries > 0 {
        retries = retries - 1;
        let _bytes_transmitted = match socket.send_to(&transmit, ntp_server) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        let mut buf = [0; 48];
        let _bytes_received = match socket.recv(&mut buf) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        let ntp_time = process_ntp_packet(&buf);
        if ntp_time >= 2208988800 { // Avoid time wrapping around
            let unix_time: u64 = ntp_time - 2208988800;
            return Ok(unix_time);
        } else {
            return Ok(0);
        }
    }
    return Err(std::io::Error::new(
        std::io::ErrorKind::TimedOut,
        "Timed out getting response from server",
    ));
}

fn process_ntp_packet(buffer: &[u8]) -> u64 {
    use byteorder::{BigEndian, ReadBytesExt};
    use std::io::{Cursor, Seek, SeekFrom};

    let mut reader = Cursor::new(buffer);
    reader.seek(SeekFrom::Start(40)).unwrap();

    let transmit_timestamp_seconds = reader.read_u32::<BigEndian>().unwrap();

    u64::from(transmit_timestamp_seconds)
}

#[cfg(test)]
mod main_tests {

    use super::*;
    use byteorder::{BigEndian, WriteBytesExt};
    use std::io::{Cursor, Seek, SeekFrom};

    #[test]
    fn unable_to_bind() {
        // Usually need to be root to bind to port 80
        let result = ntp_main("0.0.0.0:80", "pool.ntp.org:123");
        assert!(result.is_err());
    }

    #[test]
    fn response_timeout() {
        let result = ntp_main("0.0.0.0:0", "google.com:21");
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::TimedOut);
    }

    #[test]
    fn incorrect_response() {
        let test_socket = UdpSocket::bind("0.0.0.0:0").unwrap();
        let test_port = test_socket.local_addr().unwrap().port();

        let join_handle = std::thread::spawn(move || {
            let mut buf = [0; 48];
            // First test - blank response
            let (_, sender) = test_socket.recv_from(&mut buf).unwrap();
            let mut tx = [0; 48];
            test_socket.send_to(&mut tx, sender).unwrap();
            // Second test - too short
            let (_, sender) = test_socket.recv_from(&mut buf).unwrap();
            let mut tx = [0; 8];
            test_socket.send_to(&mut tx, sender).unwrap();
            // Third test - too long
            let (_, sender) = test_socket.recv_from(&mut buf).unwrap();
            let mut tx = [0; 200];
            test_socket.send_to(&mut tx, sender).unwrap();
        });

        // Test 1 - blank response
        ntp_main("0.0.0.0:0", format!("127.0.0.1:{}", test_port).as_str()).unwrap();

        // Test 2 - too short
        ntp_main("0.0.0.0:0", format!("127.0.0.1:{}", test_port).as_str()).unwrap();

        // Test 3 - too long
        ntp_main("0.0.0.0:0", format!("127.0.0.1:{}", test_port).as_str()).unwrap();

        join_handle.join().unwrap();
    }

    #[test]
    fn correct_response() {
        let test_socket = UdpSocket::bind("0.0.0.0:0").unwrap();
        let test_port = test_socket.local_addr().unwrap().port();

        let join_handle = std::thread::spawn(move || {
            let mut buf = [0; 48];
            let (_, sender) = test_socket.recv_from(&mut buf).unwrap();
            let mut cur = Cursor::new(vec![0; 48]);
            cur.seek(SeekFrom::Start(40)).unwrap();
            cur.write_u32::<BigEndian>(2208988800).unwrap();
            test_socket.send_to(cur.get_ref(), sender).unwrap();
        });

        ntp_main("0.0.0.0:0", format!("127.0.0.1:{}", test_port).as_str()).unwrap();

        join_handle.join().unwrap();
    }

    #[test]
    fn packet_decoder() {
        let mut cur = Cursor::new(vec![0; 48]);
        cur.seek(SeekFrom::Start(40)).unwrap();
        cur.write_u32::<BigEndian>(2208988800).unwrap();
        assert_eq!(process_ntp_packet(cur.get_ref()), 2208988800);
        assert_ne!(process_ntp_packet(cur.get_ref()), 10);
    }
}
