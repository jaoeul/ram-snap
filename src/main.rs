use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::io::BufReader;
use std::io::BufRead;
use std::io::BufWriter;
use std::io::Seek;
use std::io::SeekFrom;

/// Parses /proc/iomem to find the physical address ranges which belong to
/// system RAM.
fn find_ram() -> Vec<(u64, u64)> {
    let iomem = std::fs::read_to_string("/proc/iomem").unwrap();
    let lines = iomem.split("\n");
    let mut ranges: Vec<(u64, u64)> = Vec::new();
    for line in lines {
        if line.contains("System RAM") {

            let mut split = line.split("-");

            let start = split.next().unwrap();
            let end = split.next().unwrap();
            let end = end.strip_suffix(" : System RAM").unwrap();

            let start: u64 = u64::from_str_radix(start, 16).unwrap();
            let end: u64 = u64::from_str_radix(end, 16).unwrap();
            ranges.push((start, end));
        }
    }
    return ranges;
}

/// Writes the ram mapped non-zero pages of /dev/mem to `out_file` in a XMLish
/// format.
fn write_ram_to_file(out_file: &str, ranges: &Vec<(u64, u64)>) {
    let mut ram: Vec<(u64, u8)> = Vec::new();

    let mut src = File::open("/dev/mem").expect("Failed to open /dev/mem");
    let mut reader = BufReader::new(src);

    let dst = File::create(out_file).expect("Unable to create file");
    let mut writer = BufWriter::new(dst);

    let mut total_ram = 0;
    for range in ranges {
        total_ram += range.1.wrapping_sub(range.0);
    }
    println!("Total system RAM: {} / {} MiB", total_ram,
             total_ram.checked_div(1024 * 1024).unwrap());

    writer.write("<ram>".as_bytes());

    let mut nb_bytes = 0;
    for range in ranges {
        println!("Processing system RAM range: <{:#x}-{:#x}>", range.0,
                 range.1);

        let fmt_range_start = format!("\n\t<range {:#x}-{:#x}>", range.0, range.1);
        let fmt_range_end = format!("\n\t</range {:#x}-{:#x}>", range.0, range.1);

        writer.write(fmt_range_start.as_bytes());

        let mut start = range.0;
        reader.seek(SeekFrom::Start(start));

        while start < range.1 {

            let (mut end, mut tmp) = if start + 4096 >= range.1 {
                (start + 1, vec![0; 1])
            }
            else {
                (start + 4096, vec![0; 4096])
            };

            reader.read_exact(&mut tmp);

            let fmt_start = format!("\n\t\t<non-zero mem {:#x}-{:#x}>", start, end);
            let fmt_end = format!("\n\t\t</non-zero mem {:#x}-{:#x}>", start, end);

            // Do not store all-zero entries.
            if !tmp.iter().all(|&x| x == 0u8) {
                writer.write(fmt_start.as_bytes());
                writer.write(&tmp);
                writer.write(fmt_end.as_bytes());
                nb_bytes += tmp.len();
            }
            reader.consume(tmp.len());
            start += tmp.len() as u64;
        }
        println!("Range done: <{:#x}-{:#x}>", range.0, range.1);
        writer.write(fmt_range_end.as_bytes());
    }
    writer.write("\n</ram>".as_bytes());

    println!("Wrote {} bytes / {} MiB to {}", nb_bytes, nb_bytes
             .checked_div(1024 * 1024).unwrap(), out_file);
}

fn main() {
    let ranges = find_ram();
    write_ram_to_file("ram.xml", &ranges);
}
