use std::{io::Write, path::Path};

use acvm::{acir::native_types::WitnessMap, Backend};
use clap::Args;
use nargo::ops::execute_circuit;
use noirc_driver::{compile_no_check, CompileOptions};
use noirc_frontend::{graph::LOCAL_CRATE, hir::Context, node_interner::FuncId};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use crate::{
    cli::check_cmd::check_crate_and_report_errors, errors::CliError,
    resolver::resolve_root_manifest,
};

use super::NargoConfig;

/// Run the tests for this program
#[derive(Debug, Clone, Args)]
pub(crate) struct TestCommand {
    /// If given, only tests with names containing this string will be run
    test_name: Option<String>,

    #[clap(flatten)]
    compile_options: CompileOptions,
}

pub(crate) fn run<B: Backend>(
    backend: &B,
    args: TestCommand,
    config: NargoConfig,
) -> Result<(), CliError<B>> {
    let test_name: String = args.test_name.unwrap_or_else(|| "".to_owned());

    run_tests(backend, &config.program_dir, &test_name, &args.compile_options)
}

fn run_tests<B: Backend>(
    backend: &B,
    program_dir: &Path,
    test_name: &str,
    compile_options: &CompileOptions,
) -> Result<(), CliError<B>> {
    let mut context = resolve_root_manifest(program_dir)?;
    check_crate_and_report_errors(&mut context, compile_options.deny_warnings, compile_options.experimental_ssa)?;

    let test_functions = context.get_all_test_functions_in_crate_matching(&LOCAL_CRATE, test_name);
    println!("Running {} test functions...", test_functions.len());
    let mut failing = 0;

    let writer = StandardStream::stderr(ColorChoice::Always);
    let mut writer = writer.lock();

    for test_function in test_functions {
        let test_name = context.function_name(&test_function);
        writeln!(writer, "Testing {test_name}...").expect("Failed to write to stdout");
        writer.flush().ok();

        match run_test(backend, test_name, test_function, &context, compile_options) {
            Ok(_) => {
                writer.set_color(ColorSpec::new().set_fg(Some(Color::Green))).ok();
                writeln!(writer, "ok").ok();
            }
            // Assume an error was already printed to stdout
            Err(_) => failing += 1,
        }
        writer.reset().ok();
    }

    if failing == 0 {
        writer.set_color(ColorSpec::new().set_fg(Some(Color::Green))).unwrap();
        writeln!(writer, "All tests passed").ok();
    } else {
        let plural = if failing == 1 { "" } else { "s" };
        return Err(CliError::Generic(format!("{failing} test{plural} failed")));
    }

    writer.reset().ok();
    Ok(())
}

fn run_test<B: Backend>(
    backend: &B,
    test_name: &str,
    main: FuncId,
    context: &Context,
    config: &CompileOptions,
) -> Result<(), CliError<B>> {
    let program = compile_no_check(context, config, main, backend.np_language(), &|op| {
        backend.supports_opcode(op)
    })
    .map_err(|_| CliError::Generic(format!("Test '{test_name}' failed to compile")))?;

    // Run the backend to ensure the PWG evaluates functions like std::hash::pedersen,
    // otherwise constraints involving these expressions will not error.
    match execute_circuit(backend, program.circuit, WitnessMap::new()) {
        Ok(_) => Ok(()),
        Err(error) => {
            let writer = StandardStream::stderr(ColorChoice::Always);
            let mut writer = writer.lock();
            writer.set_color(ColorSpec::new().set_fg(Some(Color::Red))).ok();
            writeln!(writer, "failed").ok();
            writer.reset().ok();
            Err(error.into())
        }
    }
}
