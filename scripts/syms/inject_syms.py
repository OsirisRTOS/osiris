#!/usr/bin/env python3

from elftools.elf.elffile import ELFFile

import argparse
import os
import sys

def extract_section(file_path, section_name):
    with open(file_path, 'rb') as f:
        elf = ELFFile(f)
        section = elf.get_section_by_name(section_name)
        if section is None:
            print(f"error: section '{section_name}' not found.")
            return
        data = section.data()
        return data

def inject_section(file_path, section_name, new_data):
    with open(file_path, 'rb+') as f:
        elf = ELFFile(f)
        section = elf.get_section_by_name(section_name)
        if section is None:
            print(f"error: section '{section_name}' not found.")
            return
        data = section.data()

        if len(new_data) > len(data):
            print(f"error: new data size exceeds original section size.")
            return
        f.seek(section['sh_offset'])
        f.write(new_data)

def build_symtab_strtab_blob(symtab, strtab):
    header = len(symtab).to_bytes(4, 'little')
    blob = header + symtab + strtab
    return blob

def main():
    parser = argparse.ArgumentParser(description="Symbol patching tool for ELF files.")
    parser.add_argument("-f", "--file", help="Path to the ELF file.", required=True)

    args = parser.parse_args()

    symtab  = extract_section(args.file, ".symtab")
    strtab  = extract_section(args.file, ".strtab")
    if symtab is None or strtab is None:
        print("error: could not extract symtab or strtab sections.")
        return
    blob = build_symtab_strtab_blob(symtab, strtab)
    inject_section(args.file, ".syms_area", blob)
    print("info: symtab and strtab sections injected into .syms_area section.")

if __name__ == "__main__":
    main()

