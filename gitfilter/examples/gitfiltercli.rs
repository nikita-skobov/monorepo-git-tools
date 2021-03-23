use gumdrop::Options;
// use die::die;
use gitfilter::filter::*;

#[derive(Debug, Options, Default)]
pub struct Filter {
    // #[options(help = "Dont run anything. Just print output of what a run would do.")]
    // pub dry_run: bool,
    // #[options(help = "More detailed output")]
    // pub verbose: bool,
    #[options(short = "h", help = "Prints help information")]
    pub help: bool,
    // #[options(short = "V", help = "Prints version information")]
    // pub version: bool,
    // // thing: Option<String>,

    #[options(help = "Force git-fast-export to print out all of the blob data. This will make parsing a bit slower")]
    pub with_data: bool,

    #[options(help = "Name of branch to filter from")]
    pub branch: Option<String>,

    #[options(help = "path to filter", required)]
    pub path: String,
}


pub fn get_cli_input() -> Filter {
    let args = ::std::env::args().collect::<Vec<_>>();
    let cli = match Filter::parse_args_default(&args[1..]) {
        Err(e) => {
            println!("Failed to parse cli input: {}\n", e);
            // TODO: print usage
            //let dummy_cmd = Filter::default();
            // print_program_usage(dummy_cmd);
            std::process::exit(2);
        }
        Ok(m) => m,
    };

    cli
}

fn main() {
    use std::io::stdout;
    let filter = get_cli_input();
    // let empty_cb = |_| {
    //     if 1 == 1 {
    //         Ok(())
    //     } else {
    //         Err(())
    //     }
    // };
    // parse_git_filter_export_via_channel(filter.branch, filter.with_data, empty_cb).unwrap();


    let filter_opts = FilterOptions {
        stream: stdout(),
        branch: filter.branch,
        with_blobs: filter.with_data,
    };
    let pathrule = FilterRulePathInclude(filter.path);
    let v = vec![pathrule];

    let _ = filter_with_rules(filter_opts, v);
}
