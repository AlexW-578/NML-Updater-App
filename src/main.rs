use std::{fs, io};
use std::fs::copy;
use serde::{Deserialize, Serialize};
use serde_json;
use reqwest;
use data_encoding::HEXUPPER;
use ring::digest::{Context, Digest, SHA256};
use std::io::{BufReader, Read, Write};
use std::thread::sleep;
use std::time::Duration;
use error_chain::error_chain;
use sysinfo::{System, SystemExt};
use clap::Parser;

error_chain! {
     foreign_links {
         Io(std::io::Error);
         HttpRequest(reqwest::Error);
     }
}
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long)]
    neos_dir: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct NeosMod {
    Name: String,
    Url: String,
    NeedsUpdate: bool,
    NewVersion: String,
    OldVersion: String,
    Sha256: String,
}

fn main() {
    let args = Args::parse();
    let neos_dir = args.neos_dir;
    let json_file = format!("{0}\\nml_updater\\mods.json", &neos_dir);
    check_for_neos();
    let neos_mods: Vec<NeosMod> = load_json_from_file(json_file);
    let mods_to_update: String = user_input();
    for mod_to_update in mods_to_update.split(" ") {
        if mod_to_update.to_uppercase().contains("A") {
            update_all(neos_mods, &neos_dir);
            break;
        }
        if mod_to_update.trim().parse::<i32>().is_err() {
            println!("Entered Text is not a number");
            main();
            break;
        }
        let mut e: i32 = mod_to_update.trim().parse().unwrap();
        e -= 1;
        let neos_mod = &neos_mods[e as usize];
        println!("Updating: {}", &neos_mod.Name);
        let file_name = update_mod(&neos_mods[e as usize], &neos_dir).expect("TODO: panic message");
        let input = fs::File::open(&file_name).expect("Unable to Read File");
        let reader = BufReader::new(input);
        let digest: Digest = sha256_digest::<std::io::BufReader<std::fs::File>, Error>(reader).expect("Unable to get Sha256");
        let sha256_of_download = HEXUPPER.encode(digest.as_ref()).to_lowercase();
        let old_file = format!("{1}\\nml_mods\\{0}.dll", neos_mod.Name, neos_dir);
        if sha256_of_download == neos_mod.Sha256.to_lowercase() {
            fs::copy(&file_name, &old_file).expect("Could not copy");
            fs::remove_file(file_name).expect("Could not delete old mod");
        } else {
            println!("{3}: Sha256 is not the same.\nDownloaded: {0}\n Manifest: {1}\nPlease Manually download file at: {2}", sha256_of_download, neos_mod.Sha256.to_lowercase(), neos_mod.Url, neos_mod.Name)
        }
    }
}

fn check_for_neos() {
    let mut is_neos_running: bool = true;
    while is_neos_running {
        let s = System::new_all();
        let mut test = true;
        for process in s.processes_by_name("Neos.exe") {
            println!("Neos is running sleeping for 5 secs.");
            sleep(Duration::from_secs(5));
            test = false;
        }
        if test {
            is_neos_running = false;
            println!("Neos is not running now.")
        }
    }
}


fn update_all(neos_mods: Vec<NeosMod>, neos_dir: &String) {
    for neos_mod in neos_mods {
        println!("Updating: {}", &neos_mod.Name);
        let file_name: String = update_mod(&neos_mod, &neos_dir).expect("Unable to Download file");
        let input = fs::File::open(&file_name).expect("Unable to Read File");
        let reader = BufReader::new(input);
        let digest: Digest = sha256_digest::<std::io::BufReader<std::fs::File>, Error>(reader).expect("Unable to get Sha256");
        let sha256_of_download = HEXUPPER.encode(digest.as_ref()).to_lowercase();
        let old_file = format!("{1}\\nml_mods\\{0}.dll", neos_mod.Name, neos_dir);
        if sha256_of_download == neos_mod.Sha256.to_lowercase() {
            fs::copy(&file_name, &old_file).expect("Could not copy");
            fs::remove_file(file_name).expect("Could not delete old mod");
        } else {
            println!("{3}: Sha256 is not the same.\nDownloaded: {0}\n Manifest: {1}\nPlease Manually download file at: {2}", sha256_of_download, neos_mod.Sha256.to_lowercase(), neos_mod.Url, neos_mod.Name)
        }
    }
}

fn load_json_from_file(json_file: String) -> Vec<NeosMod> {
    let contents = fs::read_to_string(json_file).expect("Should have been able to read the file");
    let split_contents = contents.split("\n");
    let mut neos_mods: Vec<NeosMod> = Vec::new();
    let mut count = 0;
    for line in split_contents {
        if line.is_empty() { break; }
        let neos_mod: NeosMod = serde_json::from_str(line).unwrap();
        println!("{0}. {1}\nOld Version: {2} -> New Version: {3}\nURL: {4}\nSHA256: {5}\n", count + 1, neos_mod.Name, neos_mod.OldVersion, neos_mod.NewVersion, neos_mod.Url, neos_mod.Sha256.to_lowercase());
        neos_mods.insert(count, neos_mod);
        count += 1;
    }
    return neos_mods;
}

fn user_input() -> String {
    let mut mods_to_update = String::new();
    println!("Please enter the numbers of the mods you would like to update (Seperated By spaces) or `A` To Update All Of them:");
    io::stdin().read_line(&mut mods_to_update).expect("Failed to read line");
    return mods_to_update;
}

fn update_mod(neos_mod: &NeosMod, neos_dir: &String) -> std::result::Result<String, Error> {
    let response = reqwest::blocking::get(&neos_mod.Url).expect("Could not download file").bytes().expect("unable to parse bytes");
    let file_name = format!("{0}\\nml_updater\\{1}.dll", neos_dir, neos_mod.Name);
    let mut out = fs::File::create(&file_name).expect("failed to create file");
    out.write_all(&*response)?;
    Ok(file_name)
}

fn sha256_digest<R: Read, E>(mut reader: R) -> Result<Digest> {
    let mut context = Context::new(&SHA256);
    let mut buffer = [0; 1024];

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        context.update(&buffer[..count]);
    }

    Ok(context.finish())
}