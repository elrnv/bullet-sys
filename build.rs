extern crate bindgen;
extern crate cmake;

use cmake::Config;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

const LIBRARIES: [&'static str; 11] = [
	"Bullet2FileLoader",
	"Bullet3Common",
	"Bullet3Geometry",
	"BulletCollision",
	"BulletInverseDynamics",
	"LinearMath",
	"Bullet3Collision",
	"Bullet3Dynamics",
	"Bullet3OpenCL_clew",
	"BulletDynamics",
	"BulletSoftBody"];
const REPOSITORY: &'static str = "https://github.com/bulletphysics/bullet3.git";
const TAG: &'static str = "2.86.1";

macro_rules! get(($name:expr) => (ok!(env::var($name))));
macro_rules! ok(($expression:expr) => ($expression.unwrap()));
macro_rules! log {
    ($fmt:expr) => (println!(concat!("bullet-sys/build.rs:{}: ", $fmt), line!()));
    ($fmt:expr, $($arg:tt)*) => (println!(concat!("bullet-sys/build.rs:{}: ", $fmt),
    line!(), $($arg)*));
}

macro_rules! log_var(($var:ident) => (log!(concat!(stringify!($var), " = {:?}"), $var)));

fn main() {

	// Check out official source ...
	let source = PathBuf::from(&get!("CARGO_MANIFEST_DIR")).join(format!("target/source-{}", TAG));
	log_var!(source);

	// ... if it's not already checked out
	if !Path::new(&source.join(".git")).exists() {
		run("git", |command| {
			command.arg("clone")
				.arg(format!("--branch={}", TAG))
				.arg(REPOSITORY)
				.arg(&source)
		});
	}
	else // ... otherwise just pull latest changes
	{
		run("git", |command| {
			command
				.arg("-C")
				.arg(&source)
				//.arg(format!("-C", source.display()))
				.arg("pull")
				.arg("origin")
				.arg(TAG)
		});
	}
    let raw_target = env::var("TARGET").unwrap();

	// Cutoff old macOS versions here to allow support for modern features
	let target =
		if raw_target.contains("apple") {
			format!("{}16.7.0", raw_target)
		} else {
			raw_target
		};

	// Compile bullet from source since we need the C API from their examples directory
	// Disable unnecessary stuff, it takes long enough to compile already
	let out = Config::new(&source)
		.define("USE_DOUBLE_PRECISION", "OFF")
		.define("BUILD_SHARED_LIBS", "ON")
		.define("USE_SOFT_BODY_MULTI_BODY_DYNAMICS_WORLD", "OFF")
		.define("BUILD_CPU_DEMOS", "OFF")
		.define("USE_GLUT", "OFF")
		.define("BUILD_EXTRAS", "ON") // Need this for the C API
		.define("CMAKE_BUILD_TYPE", "Release")
		.target(target.as_str())
		.build();

	log_var!(out);
	let lib_dir = out.join("lib");
	log_var!(lib_dir);
	let include_dir = source.join("examples").join("SharedMemory");
	log_var!(include_dir);

	// The C_API currently lives in the examples. This may change in the future
	println!("cargo:rustc-link-search=native={}", out.display());
	for lib in LIBRARIES.iter() {
		println!("cargo:rustc-link-lib={}", lib);
	}

    // Link to libstdc++ on GNU
    if target.contains("gnu") {
        println!("cargo:rustc-link-lib=stdc++");
    }
    else if target.contains("apple") {
        println!("cargo:rustc-link-lib=c++");
    }

    println!("cargo:rerun-if-changed=build.rs");

    // The bindgen::Builder is the main entry point to bindgen, and lets build up options for the
    // resulting bindings.
    let bindings = bindgen::Builder::default()
        // Do not generate unstable Rust code that requires a nightly rustc and enabling unstable
        // features.
        //.no_unstable_rust()
        // The input header we would like to generate bindings for
        .header("c_api.h")
		.clang_arg(format!("-L{}", lib_dir.display()))
		.clang_arg(format!("-I{}", include_dir.display()))
        // Finish the builder and generate the bindings
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings.");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    //let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

fn run<F>(name: &str, mut configure: F)
    where F: FnMut(&mut Command) -> &mut Command
{
    let mut command = Command::new(name);
    let configured = configure(&mut command);
    log!("Executing {:?}", configured);
    if !ok!(configured.status()).success() {
        panic!("failed to execute {:?}", configured);
    }
    log!("Command {:?} finished successfully", configured);
}
