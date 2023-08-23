use ring::digest;
use std::fs::File;
use std::io::{self, BufReader, Read};

pub fn calc_sha256sums(path: &str) -> io::Result<digest::Digest> {
    let bin = File::open(&path)?;
    let mut context = digest::Context::new(&digest::SHA256);
    let mut reader = BufReader::new(bin);
    let mut buf = vec![0; 1024];
    loop {
        let count = reader.read(&mut buf)?;
        if count == 0 {
            break;
        }
        context.update(&buf[..count]);
    }
    Ok(context.finish())
}

#[test]
fn test_sha256_calculation() {
    let sum = "36bbe50ed96841d10443bcb670d6554f0a34b761be67ec9c4a8ad2c0c44ca42c";
    let ops = calc_sha256sums("./fixtures/shasum_testfile").unwrap();
    let ans: Vec<u8> = ring::test::from_hex(sum).unwrap();
    assert_eq!(ops.as_ref(), ans);
}

#[test]
#[should_panic]
fn test_sha256_nofile() {
    calc_sha256sums("./nothing/there").unwrap();
}
