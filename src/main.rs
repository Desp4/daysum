use std::fs::{File, OpenOptions};
use std::mem::size_of;
use std::io::prelude::*;
use chrono::prelude::*;
use std::io::SeekFrom;

fn new_file(filename: &String, time: i64) {
    let mut file = File::create(filename).unwrap();
    file.write_all(&time.to_ne_bytes()).unwrap();
}

fn insert_label(filename: &String, label: &String, timestamp: i64) {
    let mut file = OpenOptions::new().write(true).read(true).open(filename).unwrap();
    let mut time_buf = [0u8; size_of::<i64>()];

    file.read(&mut time_buf).unwrap();
    if i64::from_ne_bytes(time_buf) >= timestamp {
        panic!("Inserted timestamp is smaller than the initial file timestamp");
    }

    loop {
        match file.read(&mut time_buf){
            Ok(n) => if n != time_buf.len() {
                break; // EOF
            },
            Err(_err) => panic!("Error reading file")
        };

        if i64::from_ne_bytes(time_buf) >= timestamp {
            file.seek(SeekFrom::Current(-(size_of::<i64>() as i64))).unwrap();
            break;
        }

        let mut strsize_buf = [0u8; size_of::<usize>()];
        file.read(&mut strsize_buf).unwrap();
        file.seek(SeekFrom::Current(usize::from_ne_bytes(strsize_buf) as i64)).unwrap();
    }

    let mut file_rest = vec![0u8; 0];
    let rest_bytes = file.read_to_end(&mut file_rest).unwrap();
    file.seek(SeekFrom::Current(-(rest_bytes as i64))).unwrap();

    let str_bytes = label.as_bytes();
    file.write_all(&timestamp.to_ne_bytes()).unwrap();
    file.write_all(&str_bytes.len().to_ne_bytes()).unwrap();
    file.write_all(&str_bytes).unwrap();

    file.write_all(&file_rest[..]).unwrap();
}

fn summarize_file(filename: &String, verbose: bool) {
    let mut file = File::open(filename).unwrap();
    let mut time_buf = [0u8; size_of::<i64>()];

    file.read(&mut time_buf).unwrap();
    let first_time: i64 = i64::from_ne_bytes(time_buf);
    let mut prev_time: i64 = first_time;

    println!("{}", Local.timestamp(first_time, 0).to_rfc2822());
    if verbose {
        println!("Entries:");
    }

    let mut label_map = std::collections::HashMap::<String, i64>::new();
    loop {
        let mut strsize_buf = [0u8; size_of::<usize>()];

        match file.read(&mut time_buf){
            Ok(n) => if n != time_buf.len() {
                break; // EOF
            },
            Err(_err) => panic!("Error reading file")
        };
        let time: i64 = i64::from_ne_bytes(time_buf) - prev_time;

        file.read(&mut strsize_buf).unwrap();
        let mut str_buf = vec![0u8; usize::from_ne_bytes(strsize_buf)];
        file.read(&mut str_buf).unwrap();
        let label = String::from_utf8(str_buf).unwrap();

        if verbose == true {
            println!("{}: {}", Local.timestamp(time + prev_time, 0).to_rfc2822(), label);
        }

        label_map.entry(label).and_modify(|t| {*t += time}).or_insert(time);

        prev_time += time;
    }

    let elapsed_time: i64 = prev_time - first_time;
    let elapsed_hours: i64 = elapsed_time / 3600;
    let elapsed_seconds: i64 = (elapsed_time - elapsed_hours * 3600) / 60;
    println!("Total: {:02}:{:02}h", elapsed_hours, elapsed_seconds);

    for (label, time) in label_map.iter() {
        let hours: i64 = time / (3600);
        let minutes: i64 = (time - hours * 3600) / 60;
        let frac: f32 = *time as f32 / elapsed_time as f32;
        println!("{:16}: {:02}:{:02}h, {:.1}%", label, hours, minutes, frac * 100f32);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() == 1 && args[0] == "--help" {
        println!(
            "Usage: daysum <subcommand>\n\
            Subcommands:\n\
                \t<filename> <string> [date]       - add entry <string> to file, default date is current time\n\
                \t--help                           - show usage\n\
                \t--new <filename> [date]          - create new file, initial timestamp is [date] if provided\n\
                \t{{--sum | --sumv}} <filename>      - dump summary, optional verbose(sumv)\n\
                date is of format \"DD.MM.YYYY hh:mm\""
        );
        return;
    }
    if args.len() < 2 || args.len() > 3 {
        println!("Invalid usage, type --help for instructions");
        return;
    }

    if args[0] == "--new" {
        new_file(&args[1], 
            match args.len() {
                2 => Utc::now().timestamp(),
                _ => Local.datetime_from_str(&args[2], "%d.%m.%Y %H:%M").unwrap().timestamp()
            }
        );
    } else if args.len() == 2 && args[0] == "--sum" {
        summarize_file(&args[1], false);
    } else if args.len() == 2 && args[0] == "--sumv" {
        summarize_file(&args[1], true);
    } else {
        insert_label(&args[0], &args[1],
            match args.len() {
                2 => Utc::now().timestamp(),
                _ => Local.datetime_from_str(&args[2], "%d.%m.%Y %H:%M").unwrap().timestamp()
            }
        );
    }
}
