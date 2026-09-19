use sia32_tools::disassemble_bytes;
use std::{env,fs,process};
fn main(){
 let mut args=env::args().skip(1);
 let Some(input)=args.next() else{eprintln!("usage: siadis <input.bin> [--base 0xADDR]");process::exit(2)};
 let mut base=0u32;
 while let Some(arg)=args.next(){match arg.as_str(){
  "--base"=>{let Some(v)=args.next() else{eprintln!("--base requires value");process::exit(2)};let v=v.trim_start_matches("0x");base=u32::from_str_radix(v,16).unwrap_or_else(|_|{eprintln!("invalid base");process::exit(2)});},
  "-h"|"--help"=>{println!("usage: siadis <input.bin> [--base 0xADDR]");return},
  _=>{eprintln!("unknown argument: {arg}");process::exit(2)}
 }}
 let bytes=fs::read(&input).unwrap_or_else(|e|{eprintln!("failed to read {input}: {e}");process::exit(2)});
 print!("{}",disassemble_bytes(&bytes,base));
}
