use byteorder::{BigEndian, ReadBytesExt};
use std::io::{Cursor, Seek, SeekFrom};
use std::net::UdpSocket;

fn main() {
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();

    let mut transmit: Vec<u8> = vec![0; 48];
    transmit[0] = 0x1b;
    let _bytes_transmitted = socket.send_to(&transmit, "pool.ntp.org:123").unwrap();

    let mut buf = [0; 48];
    let _bytes_received = socket.recv(&mut buf).unwrap();

    let ntp_time = process_ntp_packet(&buf);
    let unix_time: u64 = ntp_time - 2208988800;

    dbg!(&unix_time);
}

fn process_ntp_packet(buffer: &[u8]) -> u64 {
    let mut reader = Cursor::new(buffer);
    reader.seek(SeekFrom::Start(40)).unwrap();

    let transmit_timestamp_seconds = reader.read_u32::<BigEndian>().unwrap();

    u64::from(transmit_timestamp_seconds)
}
