use std::path::PathBuf;

use object::{
    pe::{ImageDosHeader, ImageNtHeaders32, ImageNtHeaders64},
    read::pe::{ImageNtHeaders, RichHeaderInfo},
    FileKind, LittleEndian,
};

pub(super) fn run(path: PathBuf) {
    let data = std::fs::read(path).unwrap();
    match FileKind::parse(data.as_slice()).unwrap() {
        FileKind::Pe32 => print_pe::<ImageNtHeaders32>(data.as_slice()),
        FileKind::Pe64 => print_pe::<ImageNtHeaders64>(data.as_slice()),
        _ => {}
    }
}

fn print_pe<NtHeaders: ImageNtHeaders>(data: &[u8]) {
    let dos_header = ImageDosHeader::parse(data).unwrap();
    println!("DOS Header");
    println!("├ e_magic    {:X}", dos_header.e_magic.get(LittleEndian));
    println!("├ e_cblp     {:X}", dos_header.e_cblp.get(LittleEndian));
    println!("├ e_cp       {:X}", dos_header.e_cp.get(LittleEndian));
    println!("├ e_crlc     {:X}", dos_header.e_crlc.get(LittleEndian));
    println!("├ e_cparhdr  {:X}", dos_header.e_cparhdr.get(LittleEndian));
    println!("├ e_minalloc {:X}", dos_header.e_minalloc.get(LittleEndian));
    println!("├ e_maxalloc {:X}", dos_header.e_maxalloc.get(LittleEndian));
    println!("├ e_ss       {:X}", dos_header.e_ss.get(LittleEndian));
    println!("├ e_sp       {:X}", dos_header.e_sp.get(LittleEndian));
    println!("├ e_csum     {:X}", dos_header.e_csum.get(LittleEndian));
    println!("├ e_ip       {:X}", dos_header.e_ip.get(LittleEndian));
    println!("├ e_cs       {:X}", dos_header.e_cs.get(LittleEndian));
    println!("├ e_lfarlc   {:X}", dos_header.e_lfarlc.get(LittleEndian));
    println!("├ e_ovno     {:X}", dos_header.e_ovno.get(LittleEndian));
    println!("├ e_oemid    {:X}", dos_header.e_oemid.get(LittleEndian));
    println!("├ e_oeminfo  {:X}", dos_header.e_oeminfo.get(LittleEndian));
    println!("└ e_lfanew   {:X}", dos_header.e_lfanew.get(LittleEndian));

    let rich_header = RichHeaderInfo::parse(data, dos_header.nt_headers_offset() as u64).unwrap();
    println!("Rich Header");
    println!("├ offset  {:X}", rich_header.offset);
    println!("├ length  {:X}", rich_header.length);
    println!("├ xor_key {:X}", rich_header.xor_key);
    println!("└ unmasked_entries");
    for entry in rich_header.unmasked_entries() {
        println!("  ├─┬ comp_id {:X}", entry.comp_id);
        println!("  │ └ count   {:X}", entry.count);
    }

    let mut nt_headers_offset = dos_header.nt_headers_offset() as u64;
    let (nt_headers, data_directories) = NtHeaders::parse(data, &mut nt_headers_offset).unwrap();
    println!("NT Headers");
}
