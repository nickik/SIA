use sia32_tools::assemble;
use std::{env,fs,path::PathBuf,process};
fn main(){
 let mut args=env::args().skip(1);
 let Some(input)=args.next() else{eprintln!("usage: siaasm <input.sia> [-o output.bin] [--symbols]");process::exit(2)};
 let mut output=None::<PathBuf>;let mut symbols=false;
 while let Some(arg)=args.next(){match arg.as_str(){
  "-o"|"--output"=>output=args.next().map(PathBuf::from),
  "--symbols"=>symbols=true,
  "-h"|"--help"=>{println!("usage: siaasm <input.sia> [-o output.bin] [--symbols]");return},
  _=>{eprintln!("unknown argument: {arg}");process::exit(2)}
 }}
 let source=fs::read_to_string(&input).unwrap_or_else(|e|{eprintln!("failed to read {input}: {e}");process::exit(2)});
 let p=assemble(&source).unwrap_or_else(|e|{eprintln!("assembly error: {e}");process::exit(1)});
 let out=output.unwrap_or_else(||{let mut p=PathBuf::from(&input);p.set_extension("bin");p});
 fs::write(&out,&p.bytes).unwrap_or_else(|e|{eprintln!("failed to write {}: {e}",out.display());process::exit(2)});
 if symbols{for(name,address)in&p.symbols{println!("{address:08x} {name}");}}
}
