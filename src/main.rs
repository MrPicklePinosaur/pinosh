
#[macro_use]
extern crate shrs;

use std::{
    fs,
    env,
    io::{stdout, BufWriter},
    process::Command,
};

use cmd_lib::run_cmd;
use shrs::prelude::*;
use shrs_file_history::FileBackedHistoryPlugin;
use shrs_command_timer::{CommandTimerPlugin, CommandTimerState};
use shrs_mux::{MuxPlugin, MuxState};
use shrs_cd_tools::{node::NodeJs, rust::CargoToml, git::Git, DirParsePlugin, DirParseState, default_prompt};
use shrs_openai::OpenaiPlugin;
// use shrs_output_capture::OutputCapturePlugin;
// use shrs_run_context::RunContextPlugin;

fn prompt_left(line_mode: State<LineMode>) -> StyledBuf {
    let indicator = match *line_mode {
        LineMode::Insert => String::from(">").cyan(),
        LineMode::Normal => String::from(":").yellow(),
    };

    styled_buf! {" ", username().map(|u|u.with(Color::Blue)), " ", top_pwd().with(Color::White).attribute(Attribute::Bold), " ", indicator, " "}
}

fn prompt_right(cmd_timer: State<CommandTimerState>, mux: State<MuxState>, dir_parse: State<DirParseState>, shell: &Shell) -> StyledBuf {
    let time_str = cmd_timer.command_time().map(|x| format!("{x:?}"));
    let lang_name = mux.current_lang().name();

    let git_branch = dir_parse.get_module_metadata::<Git>("git")
        .map(|git| format!("git:{} ", git.branch));

    let project_indicator = default_prompt(&dir_parse, shell);

    styled_buf! {git_branch.map(|u|u.with(Color::Blue)), time_str, lang_name, project_indicator}
}

fn main() {
    env_logger::init();

    let _out = BufWriter::new(stdout());

    let config_dir = dirs::home_dir().unwrap().as_path().join(".config/pinosh");
    fs::create_dir_all(config_dir.clone());

    let mut env = Env::default();
    env.load();
    env.set("SHELL_NAME", "pinosh");

    let builtins = Builtins::default();

    let path_string = env.get("PATH").unwrap().to_string();
    let mut completer = DefaultCompleter::default();
    completer.register(Rule::new(
        Pred::new(cmdname_pred),
        Box::new(cmdname_action(path_string)),
    ));
    completer.register(Rule::new(
        Pred::new(cmdname_pred),
        Box::new(builtin_cmdname_action(&builtins)),
    ));

    let menu = DefaultMenu::default();

    // let highlighter = SyntaxHighlighter::new(SyntaxTheme::default());

    let mut bindings = Keybindings::new();
    bindings
    .insert("C-l", "Clear the screen", || -> anyhow::Result<()> {
        Command::new("clear")
            .spawn()
            .expect("Couldn't clear screen");
        Ok(())
    })
    .unwrap();
    bindings
    .insert("C-f", "Fuzzy search", || -> anyhow::Result<()> {
            
        let Ok(search_dirs) = env::var("FUZZY_DIRS") else {
            eprintln!("FUZZY_DIRS env var not specified");
            return Ok(());
        };

        // TODO not getting any output on console?
        run_cmd! (fdfind . -t d | fzf).unwrap();

        // sh.builtins.get("cd").unwrap().run(sh, ctx, rt, &vec![dir]).unwrap();
        Ok(())
    })
    .unwrap();

    let alias = Alias::from_iter([
        ("ls", "ls --color=auto"),
        ("l", "ls --color=auto"),
        ("c", "cd"),
        ("g", "git"),
        ("v", "vim"),
        ("V", "nvim"),
        ("la", "ls -a --color=auto"),
        ("t", "task"),
    ]);

    let startup_msg =
        |mut out: StateMut<OutputWriter>, _startup: &StartupCtx| -> anyhow::Result<()> {
        let welcome_str = format!(
            r#"
         _                 _     
   _ __ (_)_ __   ___  ___| |__  
  | '_ \| | '_ \ / _ \/ __| '_ \ 
  | |_) | | | | | (_) \__ \ | | |
  | .__/|_|_| |_|\___/|___/_| |_|
  |_| pinosaur's shell

  ###############################
"#,
        );

        println!("{welcome_str}");
        Ok(())
    };
    let mut hooks = Hooks::new();
    hooks.insert(startup_msg);

    let openai_api_key = std::env::var("OPENAI_KEY");

    let mut myshell = ShellBuilder::default()
        .with_menu(menu)
        .with_prompt(Prompt::from_sides(prompt_left, prompt_right))
        .with_completer(completer)
        .with_hooks(hooks)
        .with_env(env)
        .with_alias(alias)
        .with_keybindings(bindings);

    if let Ok(openai_api_key) = openai_api_key {
        myshell = myshell.with_plugin(OpenaiPlugin::new(openai_api_key));
    } else {
        println!("Missing OPENAI_KEY, skipping open_ai package");
    }

    myshell = myshell
        // .with_plugin(OutputCapturePlugin)
        .with_plugin(CommandTimerPlugin)
        .with_plugin(FileBackedHistoryPlugin::new())
        // .with_plugin(RunContextPlugin)
        .with_plugin(DirParsePlugin::new())
        .with_plugin(MuxPlugin::new());

    myshell
        .build()
        .expect("Could not construct shell")
        .run()
        .unwrap();
}
