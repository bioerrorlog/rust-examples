use libc;
use std::env;
use std::fs::{self, File};
use std::io::{self, Write};
use std::os::unix::process::CommandExt;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        panic!("Please provide a command.");
    }

    match args[1].as_str() {
        "run" => run(&args),
        "child" => child(&args),
        _ => panic!("Invalid command. Use 'run' or 'child'."),
    }
}

fn run(args: &[String]) {
    println!("Running {:?}", &args[2..]);

    let err = Command::new("/proc/self/exe")
        .arg("child")
        .args(&args[2..])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .pre_exec(|| {
            // Set up namespaces
            unsafe {
                libc::unshare(libc::CLONE_NEWUTS | libc::CLONE_NEWPID | libc::CLONE_NEWNS);
            }
            Ok(())
        })
        .spawn()
        .expect("Failed to spawn child process")
        .wait()
        .expect("Failed to wait on child process");

    must(err.success());
}

fn child(args: &[String]) {
    println!("Running {:?}", &args[2..]);

    cg();

    unsafe {
        libc::sethostname(b"container\0".as_ptr() as *const i8, "container".len());
        libc::chroot("/home/bioerrorlog/ubuntufs\0".as_ptr() as *const i8);
        libc::chdir("/\0".as_ptr() as *const i8);
    }

    mount("proc", "proc", "proc", 0, "");
    mount("thing", "mytemp", "tmpfs", 0, "");

    let err = Command::new(&args[2])
        .args(&args[3..])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .expect("Failed to execute command");

    must(err.success());

    unmount("proc");
    unmount("thing");
}

fn cg() {
    let cgroups = "/sys/fs/cgroup/";
    let pids = format!("{}pids", cgroups);
    let cg_path = format!("{}/bioerrorlog", pids);
    fs::create_dir_all(&cg_path).expect("Failed to create cgroup directory");

    let mut pids_max = File::create(format!("{}/pids.max", cg_path)).expect("Failed to set pids.max");
    pids_max.write_all(b"20").expect("Failed to write pids.max");

    let mut notify_on_release = File::create(format!("{}/notify_on_release", cg_path))
        .expect("Failed to set notify_on_release");
    notify_on_release
        .write_all(b"1")
        .expect("Failed to write notify_on_release");

    let mut cgroup_procs = File::create(format!("{}/cgroup.procs", cg_path))
        .expect("Failed to set cgroup.procs");
    cgroup_procs
        .write_all(env::current_pid().unwrap().to_string().as_bytes())
        .expect("Failed to write cgroup.procs");
}

fn mount(source: &str, target: &str, fstype: &str, flags: libc::c_ulong, data: &str) {
    unsafe {
        let res = libc::mount(
            source.as_ptr() as *const i8,
            target.as_ptr() as *const i8,
            fstype.as_ptr() as *const i8,
            flags,
            data.as_ptr() as *const libc::c_void,
        );
        if res != 0 {
            panic!("Mount failed: {}", io::Error::last_os_error());
        }
    }
}

fn unmount(target: &str) {
    unsafe {
        let res = libc::umount(target.as_ptr() as *const i8);
        if res != 0 {
            panic!("Unmount failed: {}", io::Error::last_os_error());
        }
    }
}

fn must(success: bool) {
    if !success {
        panic!("Command failed");
    }
}
