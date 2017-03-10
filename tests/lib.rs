use std::os::unix::io::{RawFd, AsRawFd};
use std::mem::size_of;
use std::iter::repeat;
// use std::io::Cursor;

extern crate crslmnl as mnl;
use mnl::linux as linux;
extern crate libc;
use libc::{socket};

fn buf_offset_as<T>(buf: &[u8], offset: isize) -> &T {
    assert!(buf.len() >= offset as usize + size_of::<T>());
    unsafe {
        (buf.as_ptr().offset(offset) as *const T).as_ref().unwrap()
    }
}

fn set_buf<T>(buf: &mut [u8], offset: isize, v: T) {
    assert!(buf.len() >= offset as usize + size_of::<T>());
    unsafe {
        *(buf.as_mut_ptr().offset(offset) as *mut T) = v;
    }
}

#[allow(dead_code)]
fn set_nlmsg_len(buf: &mut[u8], len: u32) {
    set_buf(buf, 0, len);
}

#[allow(dead_code)]
fn set_nlmsg_type(buf: &mut[u8], mtype: u16) {
    set_buf(buf, 4, mtype);
}

#[allow(dead_code)]
fn set_nlmsg_flags(buf: &mut[u8], flags: u16) {
    set_buf(buf, 6, flags);
}

#[allow(dead_code)]
fn set_nlmsg_seq(buf: &mut[u8], seq: u32) {
    set_buf(buf, 8, seq);
}

#[allow(dead_code)]
fn set_nlmsg_pid(buf: &mut[u8], pid: u32) {
    set_buf(buf, 12, pid);
}

#[test]
fn netlink_netfilter() {
    assert!(linux::netlink::Family::NETFILTER.c_int() == 12)
}

#[test]
fn socket_open() {
    assert!(mnl::Socket::open(linux::netlink::Family::NETFILTER).is_ok());
}

#[cfg(feature = "mnl-gt-1_0_4")]
#[test]
fn socket_open2() {
    assert!(mnl::Socket::open2(linux::netlink::Family::NETFILTER, 0).is_ok());
}

#[cfg(feature = "mnl-gt-1_0_4")]
#[test]
fn socket_fdopen() {
    struct RawSocket {
        fd: RawFd,
    }
    impl AsRawFd for RawSocket {
        fn as_raw_fd(&self) -> RawFd {
            self.fd
        }
    }

    // socket(AF_NETLINK, SOCK_RAW | NETLINK_NETFILTER);
    let sock = RawSocket { fd: unsafe { socket(16, 3, 12) }};
    assert!(mnl::Socket::fdopen(&sock).is_ok());
}

macro_rules! default_socket {
    () => {
        mnl::Socket::open(linux::netlink::Family::NETFILTER).unwrap()
    }
}

#[test]
fn socket_bind() {
    let nls = default_socket!();
    assert!(nls.bind(0, mnl::SOCKET_AUTOPID).is_ok());
}

#[test]
fn socket_get_fd() {
    let nls = default_socket!();
    assert!(nls.as_raw_fd() >= 0);
}

#[test]
fn socket_get_portid() {
    let nls = default_socket!();
    nls.bind(0, mnl::SOCKET_AUTOPID).unwrap();
    assert!(nls.portid() > 0);
}

// XXX: no...
//   sendto, recvfrom
//   setsockopt, getsockopt

#[test]
fn nlmsg_size() {
    assert!(mnl::Nlmsg::size(123) == 16 + 123);
}

#[test]
fn nlmsg_put_header() {
    let mut buf = vec![123 as u8; mnl::SOCKET_BUFFER_SIZE()];
    let nlh = mnl::Nlmsg::new(&mut buf);
    assert!(nlh.nlmsg_len == 16);
}

#[test]
fn nlmsg_put_extra_header() {
    let mut buf = vec![123 as u8; mnl::SOCKET_BUFFER_SIZE()];
    let nlh = mnl::Nlmsg::new(&mut buf);
    {
        let exthdr: &mut linux::netfilter::nfnetlink::Nfgenmsg
            = nlh.put_extra_header(size_of::<linux::netfilter::nfnetlink::Nfgenmsg>());
        assert!(exthdr.nfgen_family == 0);
        assert!(exthdr.version == 0);
        assert!(exthdr.res_id == 0);
    }
    assert!(nlh.nlmsg_len as usize == 16 + size_of::<linux::netfilter::nfnetlink::Nfgenmsg>());
}

#[test]
fn nlmsg_ok() {
    let mut buf = vec![0; mnl::NLMSG_HDRLEN() as usize];
    let nlh = mnl::Nlmsg::new(&mut buf);
    assert!(!nlh.ok(15));
    assert!(nlh.ok(16));
    assert!(nlh.ok(17));
}

#[test]
fn nlmsg_next_header() {
    let hdrlen = mnl::NLMSG_HDRLEN() as usize;
    let mut buf: Vec<u8> = repeat(0u8).take(512).collect();
    {
        let nlh = mnl::Nlmsg::new(&mut buf);
        let (next_nlh, rest) = nlh.next(512);
        assert!(rest == 512 - hdrlen as isize);
        assert!(next_nlh.nlmsg_len == 0);
        next_nlh.nlmsg_len = 0x11111111;
    }
    assert_eq!(buf[hdrlen..(hdrlen + 4)], [0x11, 0x11, 0x11, 0x11]);
}

#[test]
fn nlmsg_seq_ok() {
    let mut buf =vec![0u8; 512];
    {
        let nlh = mnl::Nlmsg::new(&mut buf);
        assert!(nlh.seq_ok(0));
    }
    set_nlmsg_seq(&mut buf, 1234567890);
    {
        let nlh = mnl::Nlmsg::new(&mut buf);
        assert!(nlh.seq_ok(1234567890));
    }
}

#[test]
fn nlmsg_porid_ok() {
    let mut buf =vec![0u8; 512];
    {
        let nlh = mnl::Nlmsg::new(&mut buf);
        assert!(nlh.portid_ok(0));
    }
    set_nlmsg_seq(&mut buf, 1234567890);
    {
        let nlh = mnl::Nlmsg::new(&mut buf);
        assert!(nlh.portid_ok(1234567890));
    }
}

#[test]
fn nlmsg_payload() {
    let mut buf =vec![0u8; 512];
    {
        let nlh = mnl::Nlmsg::new(&mut buf);
        let v: &mut u64 = nlh.payload_mut();
        *v = std::u64::MAX;
        assert!(*nlh.payload::<u64>() == std::u64::MAX);
        assert!(*nlh.payload_mut::<u64>() == std::u64::MAX);
    }
    assert!(*buf_offset_as::<u64>(&buf, 16) == std::u64::MAX);
}

#[test]
fn nlmsg_payload_offset() {
    let mut buf =vec![0u8; 512];
    {
        let nlh = mnl::Nlmsg::new(&mut buf);
        let v: &mut u64 = nlh.payload_offset_mut(128);
        *v = std::u64::MAX;
        assert!(*nlh.payload_offset::<u64>(128) == std::u64::MAX);
        assert!(*nlh.payload_offset_mut::<u64>(128) == std::u64::MAX);
    }
    assert!(*buf_offset_as::<u64>(&buf, 16 + 128) == std::u64::MAX);
}

#[test]
fn nlmsg_payload_tail() {
    // let nlhp: &mut mnl::Nlmsg = Default::default();
    let mut buf =vec![0u8; 512];
    set_nlmsg_len(&mut buf, 64);
    {
        let nlh = mnl::Nlmsg::from_bytes(&mut buf);
        let v: &mut u64 = nlh.payload_tail_mut();
        *v = std::u64::MAX;
        assert!(*nlh.payload_tail::<u64>() == std::u64::MAX);
        assert!(*nlh.payload_tail_mut::<u64>() == std::u64::MAX);
    }
    assert!(*buf_offset_as::<u64>(&buf, 64) == std::u64::MAX);
}

#[test]
fn nlmsg_put() {
    let mut buf = vec![0u8; 512];
    let attr_len = linux::netlink::NLA_HDRLEN() + (size_of::<u64>() as u16);
    {
        let nlh = mnl::Nlmsg::new(&mut buf);
        nlh.put(123, &std::u64::MAX);
        assert!(nlh.nlmsg_len == mnl::NLMSG_HDRLEN() + mnl::ALIGN(attr_len as u32));

        let attr = nlh.payload::<linux::netlink::Nlattr>();
        assert!(attr.nla_len == attr_len);
        assert!(attr.nla_type == 123);
    }
    assert!(*buf_offset_as::<u16>(&buf, 16) == attr_len);
    assert!(*buf_offset_as::<u16>(&buf, 18) == 123);
    assert!(*buf_offset_as::<u64>(&buf, 20) == std::u64::MAX);
}

#[test]
fn nlmsg_put_u8() {
    let mut buf = vec![0u8; 512];
    let attr_len = linux::netlink::NLA_HDRLEN() + (size_of::<u8>() as u16);
    {
        let nlh = mnl::Nlmsg::new(&mut buf);
        nlh.put_u8(12, 34);
        assert!(nlh.nlmsg_len == mnl::NLMSG_HDRLEN() + mnl::ALIGN(attr_len as u32));

        let attr = nlh.payload::<linux::netlink::Nlattr>();
        assert!(attr.nla_len == attr_len);
        assert!(attr.nla_type == 12);
    }
    assert!(*buf_offset_as::<u16>(&buf, 16) == attr_len);
    assert!(*buf_offset_as::<u16>(&buf, 18) == 12);
    assert!(*buf_offset_as::<u8>(&buf, 20) == 34);

    {
        let nlh = mnl::Nlmsg::from_bytes(&mut buf);
        let attr = nlh.payload_tail::<linux::netlink::Nlattr>();
        nlh.put_u8(56, 78);
        assert!(nlh.nlmsg_len == mnl::NLMSG_HDRLEN() + mnl::ALIGN((attr_len) as u32) * 2);

        assert!(attr.nla_len == attr_len);
        assert!(attr.nla_type == 56);
    }
    assert!(*buf_offset_as::<u16>(&buf, 24) == attr_len);
    assert!(*buf_offset_as::<u16>(&buf, 26) == 56);
    assert!(*buf_offset_as::<u8>(&buf, 28) == 78);
}

#[test]
fn nlmsg_put_u16() {
    let mut buf = vec![0u8; 512];
    let attr_len = linux::netlink::NLA_HDRLEN() + (size_of::<u16>() as u16);
    {
        let nlh = mnl::Nlmsg::new(&mut buf);
        nlh.put_u16(1234, 5678);
        assert!(nlh.nlmsg_len == mnl::NLMSG_HDRLEN() + mnl::ALIGN(attr_len as u32));

        let attr = nlh.payload::<linux::netlink::Nlattr>();
        assert!(attr.nla_len == attr_len);
        assert!(attr.nla_type == 1234);
    }
    assert!(*buf_offset_as::<u16>(&buf, 16) == attr_len);
    assert!(*buf_offset_as::<u16>(&buf, 18) == 1234);
    assert!(*buf_offset_as::<u16>(&buf, 20) == 5678);

    {
        let nlh = mnl::Nlmsg::from_bytes(&mut buf);
        let attr = nlh.payload_tail::<linux::netlink::Nlattr>();
        nlh.put_u16(9012, 3456);
        assert!(nlh.nlmsg_len == mnl::NLMSG_HDRLEN() + mnl::ALIGN((attr_len) as u32) * 2);

        assert!(attr.nla_len == attr_len);
        assert!(attr.nla_type == 9012);
    }
    assert!(*buf_offset_as::<u16>(&buf, 24) == attr_len);
    assert!(*buf_offset_as::<u16>(&buf, 26) == 9012);
    assert!(*buf_offset_as::<u16>(&buf, 28) == 3456);
}

#[test]
fn nlmsg_put_u32() {
    let mut buf = vec![0u8; 512];
    let attr_len = linux::netlink::NLA_HDRLEN() + (size_of::<u32>() as u16);
    {
        let nlh = mnl::Nlmsg::new(&mut buf);
        nlh.put_u32(1234, 56789012);
        assert!(nlh.nlmsg_len == mnl::NLMSG_HDRLEN() + mnl::ALIGN(attr_len as u32));

        let attr = nlh.payload::<linux::netlink::Nlattr>();
        assert!(attr.nla_len == attr_len);
        assert!(attr.nla_type == 1234);
    }
    assert!(*buf_offset_as::<u16>(&buf, 16) == attr_len);
    assert!(*buf_offset_as::<u16>(&buf, 18) == 1234);
    assert!(*buf_offset_as::<u32>(&buf, 20) == 56789012);

    {
        let nlh = mnl::Nlmsg::from_bytes(&mut buf);
        let attr = nlh.payload_tail::<linux::netlink::Nlattr>();
        nlh.put_u32(3456, 78901234);
        assert!(nlh.nlmsg_len == mnl::NLMSG_HDRLEN() + mnl::ALIGN((attr_len) as u32) * 2);

        assert!(attr.nla_len == attr_len);
        assert!(attr.nla_type == 3456);
    }
    assert!(*buf_offset_as::<u16>(&buf, 24) == attr_len);
    assert!(*buf_offset_as::<u16>(&buf, 26) == 3456);
    assert!(*buf_offset_as::<u32>(&buf, 28) == 78901234);
}

#[test]
fn nlmsg_put_u64() {
    let mut buf = vec![0u8; 512];
    let attr_len = linux::netlink::NLA_HDRLEN() + (size_of::<u64>() as u16);
    {
        let nlh = mnl::Nlmsg::new(&mut buf);
        nlh.put_u64(1234, 0x567890abcdef0123);
        assert!(nlh.nlmsg_len == mnl::NLMSG_HDRLEN() + mnl::ALIGN(attr_len as u32));

        let attr = nlh.payload::<linux::netlink::Nlattr>();
        assert!(attr.nla_len == attr_len);
        assert!(attr.nla_type == 1234);
    }
    assert!(*buf_offset_as::<u16>(&buf, 16) == attr_len);
    assert!(*buf_offset_as::<u16>(&buf, 18) == 1234);
    assert!(*buf_offset_as::<u64>(&buf, 20) == 0x567890abcdef0123);

    {
        let nlh = mnl::Nlmsg::from_bytes(&mut buf);
        let attr = nlh.payload_tail::<linux::netlink::Nlattr>();
        nlh.put_u64(4567, 0x890abcdef0123456);
        assert!(nlh.nlmsg_len == mnl::NLMSG_HDRLEN() + mnl::ALIGN((attr_len) as u32) * 2);

        assert!(attr.nla_len == attr_len);
        assert!(attr.nla_type == 4567);
    }
    assert!(*buf_offset_as::<u16>(&buf, 28) == attr_len);
    assert!(*buf_offset_as::<u16>(&buf, 30) == 4567);
    assert!(*buf_offset_as::<u64>(&buf, 32) == 0x890abcdef0123456);
}

#[test]
fn nlmsg_put_str() {
    let mut buf = vec![0u8; 512];
    let s1 = "Hello, world!";
    let b1 = s1.as_bytes(); // .len() == 13
    let attr_len1 = linux::netlink::NLA_HDRLEN() + (b1.len() as u16);
    {
        let nlh = mnl::Nlmsg::new(&mut buf);
        nlh.put_str(1234, s1);
        assert!(nlh.nlmsg_len == mnl::NLMSG_HDRLEN() + mnl::ALIGN(attr_len1 as u32));

        let attr = nlh.payload::<linux::netlink::Nlattr>();
        assert!(attr.nla_len == attr_len1);
        assert!(attr.nla_type == 1234);
    }
    assert!(*buf_offset_as::<u16>(&buf, 16) == attr_len1);
    assert!(*buf_offset_as::<u16>(&buf, 18) == 1234);
    assert!(std::str::from_utf8(buf_offset_as::<[u8; 13]>(&buf, 20)).unwrap() == s1);

    let s2 = "My name is";
    let b2 = s2.as_bytes(); // .len() == 10
    let attr_len2 = linux::netlink::NLA_HDRLEN() + (b2.len() as u16);
    let bi: isize;
    {
        let nlh = mnl::Nlmsg::from_bytes(&mut buf);
        let attr = nlh.payload_tail::<linux::netlink::Nlattr>();
        bi = nlh.nlmsg_len as isize;
        nlh.put_str(5678, s2);
        assert!(nlh.nlmsg_len == mnl::NLMSG_HDRLEN()
                + mnl::ALIGN(attr_len1 as u32) + mnl::ALIGN(attr_len2 as u32));

        assert!(attr.nla_len == attr_len2);
        assert!(attr.nla_type == 5678);
    }
    assert!(*buf_offset_as::<u16>(&buf, bi) == attr_len2);
    assert!(*buf_offset_as::<u16>(&buf, bi + 2) == 5678);
    assert!(std::str::from_utf8(buf_offset_as::<[u8; 10]>(&buf, bi + 4)).unwrap() == s2);
}

#[test]
fn nlmsg_put_strz() {
    let mut buf = vec![0u8; 512];
    let s1 = "Hello, world!";
    let b1 = s1.as_bytes(); // .len() == 13
    let attr_len1 = linux::netlink::NLA_HDRLEN() + (b1.len() as u16) + 1;
    let mut nlmsg_len = mnl::NLMSG_HDRLEN() + mnl::ALIGN(attr_len1 as u32);
    {
        let nlh = mnl::Nlmsg::new(&mut buf);
        nlh.put_strz(1234, s1);
        assert!(nlh.nlmsg_len == nlmsg_len);

        let attr = nlh.payload::<linux::netlink::Nlattr>();
        assert!(attr.nla_len == attr_len1);
        assert!(attr.nla_type == 1234);
    }
    assert!(*buf_offset_as::<u16>(&buf, 16) == attr_len1);
    assert!(*buf_offset_as::<u16>(&buf, 18) == 1234);
    assert!(std::str::from_utf8(buf_offset_as::<[u8; 13]>(&buf, 20)).unwrap() == s1);
    assert!(*buf_offset_as::<u8>(&buf, nlmsg_len as isize) == 0);

    let s2 = "My name is";
    let b2 = s2.as_bytes(); // .len() == 10
    let attr_len2 = linux::netlink::NLA_HDRLEN() + (b2.len() as u16) + 1;
    nlmsg_len = mnl::NLMSG_HDRLEN() + mnl::ALIGN(attr_len1 as u32) + mnl::ALIGN(attr_len2 as u32);
    let attr_tail = (mnl::NLMSG_HDRLEN() + mnl::ALIGN(attr_len1 as u32) + attr_len2 as u32 - 1) as isize;
    let bi: isize;
    {
        set_buf(&mut buf, attr_tail, 0xffu8);
        let nlh = mnl::Nlmsg::from_bytes(&mut buf);
        let attr = nlh.payload_tail::<linux::netlink::Nlattr>();
        bi = nlh.nlmsg_len as isize;
        nlh.put_strz(5678, s2);
        assert!(nlh.nlmsg_len == nlmsg_len);

        assert!(attr.nla_len == attr_len2);
        assert!(attr.nla_type == 5678);
    }
    assert!(*buf_offset_as::<u16>(&buf, bi) == attr_len2);
    assert!(*buf_offset_as::<u16>(&buf, bi + 2) == 5678);
    assert!(std::str::from_utf8(buf_offset_as::<[u8; 10]>(&buf, bi + 4)).unwrap() == s2);
    assert!(*buf_offset_as::<u8>(&buf, attr_tail) == 0);
}
