use hex;

use std::collections::HashMap;

#[cfg(windows)] use winapi::um::libloaderapi::GetModuleHandleA;

use super::*;

#[test]
fn test_compiled() {
    let pefile = VecPE::from_disk_file("test/compiled.exe").unwrap();

    assert_eq!(pefile.len(), pefile.calculate_disk_size().unwrap());

    let md5 = pefile.md5();
    assert_eq!(md5, hex::decode("4240afeb03e0fc11b72fdba7ff30dc4f").unwrap());

    let sha1 = pefile.sha1();
    assert_eq!(sha1, hex::decode("be63a89313a2d7dbf524aa4333f896a287f41a20").unwrap());

    let sha256 = pefile.sha256();
    assert_eq!(sha256, hex::decode("56202fe96d3493d03e77210d751f8e2a16ee7ee962b0ec1f6f830cce6c894540").unwrap());

    let dos_magic = &pefile[0..2];
    assert_eq!(dos_magic, &[0x4d, 0x5a]);

    let dos_stub = pefile.get_dos_stub();
    assert!(dos_stub.is_ok());
    assert_eq!(dos_stub.unwrap(), hex::decode("0E1FBA0E00B409CD21B8014CCD21546869732070726F6772616D2063616E6E6F742062652072756E20696E20444F53206D6F64652E0D0D0A24000000000000005D5C6DC1193D0392193D0392193D0392972210921E3D0392E51D1192183D039252696368193D03920000000000000000").unwrap());
    
    let arch = pefile.get_arch();
    assert!(arch.is_ok());
    assert_eq!(arch.unwrap(), Arch::X86);

    let bad_header = pefile.get_valid_nt_headers_64();
    assert!(bad_header.is_err());

    let mz_check = Offset(0).as_ptr(&pefile);
    assert!(mz_check.is_ok());
    assert_eq!(unsafe { *(mz_check.unwrap() as *const u16) }, DOS_SIGNATURE);

    let entry_check = Offset(0x400).as_ptr(&pefile);
    assert!(entry_check.is_ok());
    assert_eq!(unsafe { *entry_check.unwrap() }, 0x68);

    let e_lfanew_check = pefile.e_lfanew();
    assert!(e_lfanew_check.is_ok());

    let e_lfanew = e_lfanew_check.unwrap();

    let search = pefile.search_ref(&NT_SIGNATURE);
    assert!(search.is_ok());
    assert_eq!(search.unwrap().collect::<Vec<usize>>(), vec![e_lfanew.into()]);

    let get_headers = pefile.get_valid_nt_headers_32();
    assert!(get_headers.is_ok());

    let headers = get_headers.unwrap();
    
    let get_section_table = pefile.get_section_table();
    assert!(get_section_table.is_ok());

    let section_table = get_section_table.unwrap();
    assert_eq!(section_table.len(), headers.file_header.number_of_sections as usize);
    assert_eq!(section_table[0].name.as_str().unwrap(), ".text");
    assert_eq!(section_table[1].name.as_str().unwrap(), ".rdata");
    assert_eq!(section_table[2].name.as_str().unwrap(), ".data");

    let data_directory_offset = pefile.get_data_directory_offset();
    assert!(data_directory_offset.is_ok());
    assert_eq!(data_directory_offset.unwrap(), Offset(0x128));

    assert!(pefile.has_data_directory(ImageDirectoryEntry::Import));
    assert!(!pefile.has_data_directory(ImageDirectoryEntry::Export));
    
    let import_directory_result = ImportDirectory::parse(&pefile);
    assert!(import_directory_result.is_ok());

    let import_directory = import_directory_result.unwrap();
    assert_eq!(import_directory.descriptors.len(), 2);
    assert_eq!(import_directory.descriptors[0].original_first_thunk, RVA(0x2040));
    assert_eq!(import_directory.descriptors[0].name, RVA(0x20A0));
    assert_eq!(import_directory.descriptors[0].first_thunk, RVA(0x2080));

    let name_0 = import_directory.descriptors[0].get_name(&pefile);
    assert!(name_0.is_ok());
    assert_eq!(name_0.unwrap().as_str().unwrap(), "kernel32.dll");

    let kernel32_thunks_result = import_directory.descriptors[0].get_original_first_thunk(&pefile);
    assert!(kernel32_thunks_result.is_ok());

    let kernel32_thunks = kernel32_thunks_result.unwrap();
    if let Thunk::Thunk32(kernel32_thunk) = kernel32_thunks[0] {
        assert_eq!(*kernel32_thunk, Thunk32(0x2060));
    }
    else {
        panic!("bad thunk");
    }
        
    let kernel32_imports = import_directory.descriptors[0].get_imports(&pefile);
    let kernel32_expected = vec![ImportData::ImportByName("ExitProcess")];
    assert!(kernel32_imports.is_ok());
    assert_eq!(kernel32_imports.unwrap(), kernel32_expected);

    let name_1 = import_directory.descriptors[1].get_name(&pefile);
    assert!(name_1.is_ok());
    assert_eq!(name_1.unwrap().as_str().unwrap(), "msvcrt.dll");

    let msvcrt_imports = import_directory.descriptors[1].get_imports(&pefile);
    let msvcrt_expected = vec![ImportData::ImportByName("printf")];
    assert!(msvcrt_imports.is_ok());
    assert_eq!(msvcrt_imports.unwrap(), msvcrt_expected);

    let known_mem_image = std::fs::read("test/compiled_dumped.bin").unwrap();
    let recreated_image = pefile.recreate_image(PEType::Memory);
    assert!(recreated_image.is_ok());
    
    // due to the IAT, the images are not equal by a few bytes, so we instead
    // compare the length of the two images, which should be equal if it properly
    // recreated the image.
    assert_eq!(known_mem_image.len(), recreated_image.unwrap().len());
}

#[test]
fn test_compiled_dumped() {
    let pefile = VecPE::from_memory_file("test/compiled_dumped.bin").unwrap();

    assert_eq!(pefile.len(), pefile.calculate_memory_size().unwrap());

    let dos_stub = pefile.get_dos_stub();
    assert!(dos_stub.is_ok());
    assert_eq!(dos_stub.unwrap(), hex::decode("0E1FBA0E00B409CD21B8014CCD21546869732070726F6772616D2063616E6E6F742062652072756E20696E20444F53206D6F64652E0D0D0A24000000000000005D5C6DC1193D0392193D0392193D0392972210921E3D0392E51D1192183D039252696368193D03920000000000000000").unwrap());

    let arch = pefile.get_arch();
    assert!(arch.is_ok());
    assert_eq!(arch.unwrap(), Arch::X86);

    let bad_header = pefile.get_valid_nt_headers_64();
    assert!(bad_header.is_err());

    let mz_check = Offset(0).as_ptr(&pefile); // should be fine after translation as Offset(RVA(0))
    assert!(mz_check.is_ok());
    assert_eq!(unsafe { *(mz_check.unwrap() as *const u16) }, DOS_SIGNATURE);

    let entry_check = Offset(0x400).as_ptr(&pefile); // should translate to Offset(RVA(0x1000))
    assert!(entry_check.is_ok());
    assert_eq!(unsafe { *entry_check.unwrap() }, 0x68);

    let get_headers = pefile.get_valid_nt_headers_32();
    assert!(get_headers.is_ok());

    let headers = get_headers.unwrap();
    
    let get_section_table = pefile.get_section_table();
    assert!(get_section_table.is_ok());

    let section_table = get_section_table.unwrap();
    assert_eq!(section_table.len(), headers.file_header.number_of_sections as usize);
    assert_eq!(section_table[0].name.as_str().unwrap(), ".text");
    assert_eq!(section_table[1].name.as_str().unwrap(), ".rdata");
    assert_eq!(section_table[2].name.as_str().unwrap(), ".data");

    assert!(pefile.has_data_directory(ImageDirectoryEntry::Import));
    assert!(!pefile.has_data_directory(ImageDirectoryEntry::Export));

    let import_directory_result = ImportDirectory::parse(&pefile);
    assert!(import_directory_result.is_ok());

    let import_directory = import_directory_result.unwrap();
    assert_eq!(import_directory.descriptors.len(), 2);
    assert_eq!(import_directory.descriptors[0].original_first_thunk, RVA(0x2040));
    assert_eq!(import_directory.descriptors[0].name, RVA(0x20A0));
    assert_eq!(import_directory.descriptors[0].first_thunk, RVA(0x2080));

    let name_0 = import_directory.descriptors[0].get_name(&pefile);
    assert!(name_0.is_ok());
    assert_eq!(name_0.unwrap().as_str().unwrap(), "kernel32.dll");

    let kernel32_thunks_result = import_directory.descriptors[0].get_original_first_thunk(&pefile);
    assert!(kernel32_thunks_result.is_ok());

    let kernel32_thunks = kernel32_thunks_result.unwrap();
    if let Thunk::Thunk32(kernel32_thunk) = kernel32_thunks[0] {
        assert_eq!(*kernel32_thunk, Thunk32(0x2060));
    }
    else {
        panic!("bad thunk");
    }
        
    let kernel32_imports = import_directory.descriptors[0].get_imports(&pefile);
    let kernel32_expected = vec![ImportData::ImportByName("ExitProcess")];
    assert!(kernel32_imports.is_ok());
    assert_eq!(kernel32_imports.unwrap(), kernel32_expected);

    let name_1 = import_directory.descriptors[1].get_name(&pefile);
    assert!(name_1.is_ok());
    assert_eq!(name_1.unwrap().as_str().unwrap(), "msvcrt.dll");

    let msvcrt_imports = import_directory.descriptors[1].get_imports(&pefile);
    let msvcrt_expected = vec![ImportData::ImportByName("printf")];
    assert!(msvcrt_imports.is_ok());
    assert_eq!(msvcrt_imports.unwrap(), msvcrt_expected);

    let known_disk_image = std::fs::read("test/compiled.exe").unwrap();
    let recreated_image = pefile.recreate_image(PEType::Disk);
    assert!(recreated_image.is_ok());
    assert_eq!(known_disk_image.len(), recreated_image.unwrap().len());
}

#[test]
fn test_dll() {
    let pefile = VecPE::from_disk_file("test/dll.dll").unwrap();

    assert!(pefile.has_data_directory(ImageDirectoryEntry::Export));

    let directory = ExportDirectory::parse(&pefile);
    assert!(directory.is_ok());

    let export_table = directory.unwrap();
    let name = export_table.get_name(&pefile);
    assert!(name.is_ok());
    assert_eq!(name.unwrap().as_str().unwrap(), "dll.dll");

    let exports = export_table.get_export_map(&pefile);
    let expected: HashMap<&str, ThunkData> = [("export", ThunkData::Function(RVA(0x1024)))].iter().map(|&x| x).collect();

    assert!(exports.is_ok());
    assert_eq!(exports.unwrap(), expected);
    assert!(pefile.has_data_directory(ImageDirectoryEntry::BaseReloc));

    let relocation_directory_result = RelocationDirectory::parse(&pefile);
    assert!(relocation_directory_result.is_ok());

    let relocation_table = relocation_directory_result.unwrap();
    assert_eq!(relocation_table.entries.len(), 1);

    let relocation_data = relocation_table.relocations(&pefile, 0x02000000);
    let expected: Vec<(RVA, RelocationValue)> = [
        (RVA(0x1008), RelocationValue::Relocation32(0x02001059)),
        (RVA(0x100F), RelocationValue::Relocation32(0x02001034)),
        (RVA(0x1017), RelocationValue::Relocation32(0x020010D0)),
        (RVA(0x1025), RelocationValue::Relocation32(0x0200107E)),
        (RVA(0x102B), RelocationValue::Relocation32(0x020010D0)),
    ].iter().cloned().collect();

    assert!(relocation_data.is_ok());
    assert_eq!(relocation_data.unwrap(), expected);
}

#[test]
fn test_dll_fw() {
    let pefile = VecPE::from_disk_file("test/dllfw.dll").unwrap();

    assert!(pefile.has_data_directory(ImageDirectoryEntry::Export));

    let directory = ExportDirectory::parse(&pefile);
    assert!(directory.is_ok());

    let export_table = directory.unwrap();
    let exports = export_table.get_export_map(&pefile);
    let expected: HashMap<&str, ThunkData> = [("ExitProcess", ThunkData::ForwarderString(RVA(0x1060)))].iter().map(|&x| x).collect();
    assert!(exports.is_ok());

    let export_map = exports.unwrap();
    assert_eq!(export_map, expected);

    if let ThunkData::ForwarderString(forwarder_rva) = export_map["ExitProcess"] {
        let forwarder_offset = forwarder_rva.as_offset(&pefile);
        assert!(forwarder_offset.is_ok());

        let offset = forwarder_offset.unwrap();
        let string_data = pefile.get_cstring(offset.into(), false, None);
        assert!(string_data.is_ok());
        assert_eq!(string_data.unwrap().as_str().unwrap(), "msvcrt.printf");
    }
    else {
        panic!("couldn't get forwarder string");
    }
}

#[test]
fn test_imports_nothunk() {
    let pefile = VecPE::from_disk_file("test/imports_nothunk.exe").unwrap();

    assert!(pefile.has_data_directory(ImageDirectoryEntry::Import));

    let data_directory = ImportDirectory::parse(&pefile);
    assert!(data_directory.is_ok());

    let import_table = data_directory.unwrap();
    assert_eq!(import_table.descriptors.len(), 3);

    let kernel32_imports = import_table.descriptors[0].get_imports(&pefile);
    assert!(kernel32_imports.is_ok());
    assert_eq!(kernel32_imports.unwrap(), [ImportData::ImportByName("ExitProcess")]);

    let blank_imports = import_table.descriptors[1].get_imports(&pefile);
    assert!(blank_imports.is_ok());
    assert!(blank_imports.unwrap().is_empty());

    let msvcrt_imports = import_table.descriptors[2].get_imports(&pefile);
    assert!(msvcrt_imports.is_ok());
    assert_eq!(msvcrt_imports.unwrap(), [ImportData::ImportByName("printf")]);
}

#[test]
fn test_no_dd() {
    let pefile = VecPE::from_disk_file("test/no_dd.exe").unwrap();

    let data_directory = pefile.get_data_directory_table();
    assert!(data_directory.is_ok());
    assert!(data_directory.unwrap().is_empty());
}

#[test]
fn test_hello_world() {
    let pefile = VecPE::from_disk_file("test/hello_world.exe").unwrap();

    let debug_directory_check = DebugDirectory::parse(&pefile);
    assert!(debug_directory_check.is_ok());

    let debug_directory = debug_directory_check.unwrap();
    assert_eq!(ImageDebugType::from_u32(debug_directory.type_), ImageDebugType::CodeView);
}

#[test]
fn test_hello_world_packed() {
    let pefile = VecPE::from_disk_file("test/hello_world_packed.exe").unwrap();

    let entropy = pefile.entropy();
    assert!(entropy > 7.0);
    assert!(pefile.has_data_directory(ImageDirectoryEntry::Resource));

    let data_directory = ResourceDirectory::parse(&pefile);
    assert!(data_directory.is_ok());

    let resource_table = data_directory.unwrap();
    assert_eq!(resource_table.resources.len(), 1);

    let rsrc = &resource_table.resources[0];

    assert_eq!(rsrc.type_id, ResolvedDirectoryID::ID(24));
    assert_eq!(rsrc.rsrc_id, ResolvedDirectoryID::ID(1));
    assert_eq!(rsrc.lang_id, ResolvedDirectoryID::ID(1033));
    assert_eq!(rsrc.data, ResourceOffset(0x48));
}

#[test]
fn test_hello_world_rust() {
    let pefile = VecPE::from_disk_file("test/hello_world_rust.exe").unwrap();

    let tls_directory_check = TLSDirectory::parse(&pefile);
    assert!(tls_directory_check.is_ok());

    if let TLSDirectory::TLS64(tls_directory) = tls_directory_check.unwrap() {
        let raw_data = tls_directory.read(&pefile);
        assert!(raw_data.is_ok());
        assert_eq!(raw_data.unwrap(), vec![0u8; tls_directory.get_raw_data_size()].as_slice());

        let callbacks = tls_directory.get_callbacks(&pefile);
        assert!(callbacks.is_ok());
        assert_eq!(callbacks.unwrap(), &[VA64(0x14000cf00)]);
    }
    else {
        panic!("couldn't get TLS directory");
    }
}

#[test]
fn test_cff_explorer() {
    let pefile = VecPE::from_disk_file("test/cff_explorer.exe").unwrap();

    let checksum = pefile.validate_checksum();
    assert!(checksum.is_ok());
    assert!(checksum.unwrap());

    let imphash = pefile.calculate_imphash();
    assert!(imphash.is_ok());
    assert_eq!(imphash.unwrap(), hex::decode("29307ef77ea94259e99f987498998a8f").unwrap());

    assert!(pefile.has_data_directory(ImageDirectoryEntry::Resource));

    let data_directory = ResourceDirectory::parse(&pefile);
    assert!(data_directory.is_ok());

    let resource_table = data_directory.unwrap();
    let cursors = resource_table.filter(Some(ResolvedDirectoryID::ID(ResourceID::Cursor as u32)), None, None);
    assert_eq!(cursors.len(), 17);

    let bitmaps = resource_table.filter(Some(ResolvedDirectoryID::ID(ResourceID::Bitmap as u32)), None, None);
    assert_eq!(bitmaps.len(), 30);

    let icons = resource_table.filter(Some(ResolvedDirectoryID::ID(ResourceID::Icon as u32)), None, None);
    assert_eq!(icons.len(), 43);

    let fonts = resource_table.filter(Some(ResolvedDirectoryID::ID(ResourceID::Font as u32)), None, None);
    assert_eq!(fonts.len(), 0);

    let vs_version_check = VSVersionInfo::parse(&pefile);
    assert!(vs_version_check.is_ok());

    let vs_version = vs_version_check.unwrap();

    if let Some(string_file_info) = vs_version.string_file_info {
        let lang_key = string_file_info.children[0].key_as_u32();
        assert!(lang_key.is_ok());
        assert_eq!(lang_key.unwrap(), 0x40904e4);

        let lang_id = string_file_info.children[0].get_lang_id();
        assert!(lang_id.is_ok());
        assert_eq!(lang_id.unwrap(), 0x409);

        let codepage = string_file_info.children[0].get_code_page();
        assert!(codepage.is_ok());
        assert_eq!(codepage.unwrap(), 0x4e4);

        let expected: HashMap<String, String> = [("ProductVersion".to_string(), "8.0.0.0".to_string()),
                                                 ("OriginalFilename".to_string(), "CFF Explorer.exe".to_string()),
                                                 ("FileDescription".to_string(), "Common File Format Explorer".to_string()),
                                                 ("FileVersion".to_string(), "8.0.0.0".to_string()),
                                                 ("ProductName".to_string(), "CFF Explorer".to_string()),
                                                 ("CompanyName".to_string(), "Daniel Pistelli".to_string()),
                                                 ("InternalName".to_string(), "CFF Explorer.exe".to_string()),
                                                 ("LegalCopyright".to_string(), "© 2012 Daniel Pistelli.  All rights reserved.".to_string())
        ].iter().cloned().collect();
        let string_map = string_file_info.children[0].string_map();
        assert_eq!(string_map.unwrap(), expected);
    }
    else {
        panic!("couldn't get string file info");
    }
}

#[test]
fn test_creation() {
    // this test creates a working executable that simply exits with 0 as the return code.
    // simply add `created_file.save_as("my_file.exe")` to the end of the function to see for yourself
    let mut created_file = VecPE::new_disk(0x400);

    let dos_result = created_file.write_ref(0, &ImageDOSHeader::default());
    assert!(dos_result.is_ok());

    let e_lfanew = created_file.e_lfanew();
    assert!(e_lfanew.is_ok());
    assert_eq!(e_lfanew.unwrap(), Offset(0xE0));

    let nt_result = created_file.write_ref(created_file.e_lfanew().unwrap().into(), &ImageNTHeaders64::default());
    assert!(nt_result.is_ok());

    let nt_headers = created_file.get_valid_nt_headers();
    assert!(nt_headers.is_ok());

    if let NTHeaders::NTHeaders64(nt_headers_64) = nt_headers.unwrap() {
        assert_eq!(nt_headers_64.file_header.number_of_sections, 0);
    }
    else {
        panic!("couldn't get mutable NT headers");
    }

    let mut new_section = ImageSectionHeader::default();
    
    new_section.set_name(Some(".text"));
    assert_eq!(new_section.name.as_str().unwrap(), ".text");

    let created_section_check = created_file.append_section(&new_section);
    assert!(created_section_check.is_ok());

    let created_section = created_section_check.unwrap();
    assert_eq!(created_section.pointer_to_raw_data, Offset(0x400));
    assert_eq!(created_section.virtual_address, RVA(0x1000));

    let data: &[u8] = &[0x48, 0x31, 0xC0, 0xC3]; // xor rax,rax / ret
    
    created_section.virtual_size = 0x1000;
    created_section.size_of_raw_data = data.len() as u32;
    created_section.characteristics = SectionCharacteristics::MEM_EXECUTE
        | SectionCharacteristics::MEM_READ
        | SectionCharacteristics::CNT_CODE;

    // clone the section to stop mutable borrowing of created_file
    let cloned_section = created_section.clone();
    assert!(cloned_section.is_aligned_to_file(&created_file));
    assert!(cloned_section.is_aligned_to_section(&created_file));
    
    let nt_headers = created_file.get_valid_mut_nt_headers();
    assert!(nt_headers.is_ok());

    if let NTHeadersMut::NTHeaders64(nt_headers_64) = nt_headers.unwrap() {
        assert_eq!(nt_headers_64.file_header.number_of_sections, 1);
        nt_headers_64.optional_header.address_of_entry_point = RVA(0x1000);
        nt_headers_64.optional_header.size_of_headers = 0x200;
    }
    else {
        panic!("couldn't get NT headers");
    }

    let section_table = created_file.get_section_table();
    assert!(section_table.is_ok());
    assert_eq!(section_table.unwrap()[0], cloned_section);

    let section_offset = cloned_section.data_offset(created_file.get_type());
    assert_eq!(section_offset, 0x400);

    let append_offset = created_file.len();
    assert_eq!(append_offset, section_offset);

    created_file.append(&mut data.to_vec());

    let read_result = cloned_section.read(&created_file);
    assert!(read_result.is_ok());
    assert_eq!(read_result.unwrap(), data);

    let alt_read_result = created_file.read(section_offset.into(), data.len());
    assert!(alt_read_result.is_ok());
    assert_eq!(alt_read_result.unwrap(), data);

    assert!(created_file.pad_to_alignment().is_ok());
    assert!(created_file.fix_image_size().is_ok());
}

#[cfg(windows)]
#[test]
fn test_pointer() {
    let hmodule = unsafe { GetModuleHandleA(std::ptr::null()) };
    let memory_module = unsafe { PtrPE::from_memory(hmodule as *const u8) };
    assert!(memory_module.is_ok());
}

#[test]
fn test_add_relocation() {
    let pefile_ro = VecPE::from_disk_file("test/dll.dll").unwrap();
    let mut pefile = pefile_ro.clone();

    let mut relocation_directory = RelocationDirectory::parse(&pefile_ro).unwrap();
    let add_result = relocation_directory.add_relocation(&mut pefile, RVA(0x11C0));
    assert!(add_result.is_ok());

    let reparsed = RelocationDirectory::parse(&pefile).unwrap();
    let relocations = reparsed.relocations(&pefile, 0x02000000).unwrap();
    assert_eq!(relocations[0], (RVA(0x11C0), RelocationValue::Relocation32(0x01000000)));
}

#[test]
fn test_load_flareon() {
    let flareon = VecPE::from_disk_file("test/flareon.noaslr.exe").unwrap();
    let mut flareon_loaded = VallocPE::from_pe(&flareon).unwrap();
    assert!(flareon_loaded.load_image().is_ok());
}

#[test]
fn test_bakunawa() {
    let bakunawa = VecPE::from_disk_file("test/bakunawa.exe").unwrap();
    let resources = ResourceDirectory::parse(&bakunawa).unwrap();
    let search = resources.filter(Some(ResolvedDirectoryID::Name("CHIPTUNE".into())), None, None);

    assert!(search.len() == 6);

    let groups_check = resources.icon_groups(&bakunawa);
    assert!(groups_check.is_ok());

    if groups_check.is_ok() {
        let groups = groups_check.unwrap();
        assert!(groups.len() == 1);

        for (id, grp) in &groups {
            let buffer_check = grp.to_icon_buffer(&bakunawa);

            assert!(buffer_check.is_ok());

            if buffer_check.is_ok() {
                let buffer = buffer_check.unwrap();
                let known_hash = hex::decode("cce8557e73622fe5b2c93216578fd3bfece8468ba27e76c7b6360f9a686e606e").unwrap();
                let calc_hash = buffer.as_slice().sha256();
                assert!(known_hash == calc_hash);
            }
        }
    }
}
