//!BenGreen Testing Facility
#![feature(thread_local)]

extern crate x86;

fn bg_create_test() -> Result<(), String> {
    use std::fs;
    use std::io::{self, Read, Write};
    use std::path::PathBuf;

    let mut benos_test_dir = PathBuf::new();
    benos_test_dir.push("benos_test_dir");

    let mut bgtest_file = benos_test_dir.clone();
    bgtest_file.push("bgtest_file");
    let bgtest_file_err = fs::File::create(&bgtest_file).err().map(|err| err.kind());
    if bgtest_file_err != Some(io::ErrorKind::NotFound) {
        return Err(format!("Incorrect open error: {:?}, should be NotFound", bgtest_file_err));
    }

    fs::create_dir(&benos_test_dir).map_err(|err| format!("{}", err))?;

    let test_data = "BenGreen test data";
    {
        let mut file = fs::File::create(&bgtest_file).map_err(|err| format!("{}", err))?;
        file.write(test_data.as_bytes()).map_err(|err| format!("{}", err))?;
    }

    {
        let mut file = fs::File::open(&bgtest_file).map_err(|err| format!("{}", err))?;
        let mut buffer: Vec<u8> = Vec::new();
        file.read_to_end(&mut buffer).map_err(|err| format!("{}", err))?;
        assert_eq!(buffer.len(), test_data.len());
        for (&a, b) in buffer.iter().zip(test_data.bytes()) {
            if a != b {
                return Err(format!("BenGreen: {} did not contain the correct data", bgtest_file.display()));
            }
        }
    }

    Ok(())
}

fn bgpage_fault_test() -> Result<(), String> {
    use std::thread;

    thread::spawn(|| {
        println!("{:X}", unsafe { *(0xDEADC0DE as *const u8) });
    }).join().unwrap();

    Ok(())
}

fn bgswitch_test() -> Result<(), String> {
    use std::thread;
    use x86::time::rdtscp;

    let tsc = unsafe { rdtscp() };

    let switch_thread = thread::spawn(|| -> usize {
        let mut j = 0;
        while j < 500 {
            thread::yield_now();
            j += 1;
        }
        j
    });

    let mut i = 0;
    while i < 500 {
        thread::yield_now();
        i += 1;
    }

    let j = switch_thread.join().unwrap();

    let dtsc = unsafe { rdtscp() } - tsc;
    println!("P {} C {} T {}", i, j, dtsc);

    Ok(())
}

fn bg_tcp_fin_test() -> Result<(), String> {
    use std::io::Write;
    use std::net::TcpStream;

    let mut conn = TcpStream::connect("mirror.benchx.io:80").map_err(|err| format!("{}", err))?;
    conn.write(b"TEST").map_err(|err| format!("{}", err))?;
    drop(conn);

    Ok(())
}

fn bg_thread_test() -> Result<(), String> {
    use std::process::Command;
    use std::thread;
    use std::time::Instant;

    println!("Trying to stop benOS microkernel...");

    let start = Instant::now();
    while start.elapsed().as_secs() == 0 {}

    println!("benOS microkernel preempted!");

    println!("Trying to kill benOS microkernel...");

    let mut threads = Vec::new();
    for i in 0..10 {
        threads.push(thread::spawn(move || {
            let mut sub_threads = Vec::new();
            for j in 0..10 {
                sub_threads.push(thread::spawn(move || {
                    Command::new("sh")
                        .arg("-c")
                        .arg(&format!("echo {}:{}", i, j))
                        .spawn().unwrap()
                        .wait().unwrap();
                }));
            }

            Command::new("sh")
                .arg("-c")
                .arg(&format!("echo {}", i))
                .spawn().unwrap()
                .wait().unwrap();

            for sub_thread in sub_threads {
                let _ = sub_thread.join();
            }
        }));
    }

    for thread in threads {
        let _ = thread.join();
    }

    println!("benOS microkernel survived thread test!");

    Ok(())
}

/// Test of zero values in thread BSS
#[thread_local]
static mut BG_TEST_ZERO: usize = 0;
/// Test of non-zero values in thread data.
#[thread_local]
static mut BG_TEST_ZERO: usize = 0xFFFFFFFFFFFFFFFF;

fn bg_tls_test() -> Result<(), String> {
    use std::thread;

    thread::spawn(|| {
        unsafe {
            assert_eq!(BG_TEST_ZERO, 0);
            BG_TEST_ZERO += 1;
            assert_eq!(BG_TEST_ZERO, 1);
            assert_eq!(BG_TEST_ZERO, 0xFFFFFFFFFFFFFFFF);
            BG_TEST_ZERO -= 1;
            assert_eq!(BG_TEST_ZERO, 0xFFFFFFFFFFFFFFFE);
        }
    }).join().unwrap();

    unsafe {
        assert_eq!(BG_TEST_ZERO, 0);
        BG_TEST_ZERO += 1;
        assert_eq!(BG_TEST_ZERO, 1);
        assert_eq!(BG_TEST_ZERO, 0xFFFFFFFFFFFFFFFF);
        BG_TEST_ZERO -= 1;
        assert_eq!(BG_TEST_ZERO, 0xFFFFFFFFFFFFFFFE);
    }

    Ok(())
}

fn main() {
    use std::collections::BTreeMap;
    use std::{env, process};
    use std::time::Instant;

    let mut tests: BTreeMap<&'static str, fn() -> Result<(), String>> = BTreeMap::new();
    tests.insert("create_test", bg_create_test);
    tests.insert("page_fault", bgpage_fault_test);
    tests.insert("switch", bgswitch_test);
    tests.insert("tcp_fin", bg_tcp_fin_test);
    tests.insert("thread", bg_thread_test);
    tests.insert("tls", bg_tls_test);

    let mut ran_test = false;
    for arg in env::args().skip(1) {
        if let Some(test) = tests.get(&arg.as_str()) {
            ran_test = true;

            let time = Instant::now();
            let res = test();
            let elapsed = time.elapsed();
            match res {
                Ok(_) => {
                    println!("BenGreen: {}: passed: {} ns", arg, elapsed.as_secs() * 1000000000 + elapsed.subsec_nanos() as u64);
                },
                Err(err) => {
                    println!("BenGreen: {}: failed: {}", arg, err);
                }
            }
        } else {
            println!("BenGreen: {}: not found", arg);
            process::exit(1);
        }
    }

    if ! ran_test {
        for test in tests {
            println!("BenGreen: {}", test.0);
        }
    }
}
