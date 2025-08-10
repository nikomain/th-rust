use colored::*;
use console::Term;
use crossterm::{execute, terminal::{Clear, ClearType}};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, Write};
use std::time::Duration;
use tokio::time::sleep;
use tokio::task;
use rand::Rng;

/// Clear the terminal screen
pub fn clear_screen() -> io::Result<()> {
    let mut stdout = io::stdout();
    execute!(stdout, Clear(ClearType::All))?;
    print!("\x1B[H"); // Move cursor to top-left
    stdout.flush()?;
    Ok(())
}

/// Print a loading animation for async operations
pub async fn show_loading<F, R>(message: &str, operation: F) -> R
where
    F: std::future::Future<Output = R>,
{
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{spinner:.blue} {msg}")
            .expect("Failed to create spinner template"),
    );
    spinner.set_message(message.to_string());
    spinner.enable_steady_tick(Duration::from_millis(80));

    let result = operation.await;
    
    spinner.finish_and_clear();
    result
}

pub async fn load_content<F, R>(message: &str, operation: F) -> R
where
    F: std::future::Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    use tokio::sync::oneshot;

    let wave_chars = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
    let wave_len = 63;

    let (tx, mut rx) = oneshot::channel();

    // Spawn the operation
    task::spawn(async move {
        let result = operation.await;
        let _ = tx.send(result);
    });

    let mut pos: i32 = 0;
    let mut direction: i32 = 1;

    // Hide cursor
    print!("\x1b[?25l");
    io::stdout().flush().unwrap();

    loop {
        let mut line = String::new();
        let msg_len = message.len();
        let padded_msg_len = msg_len + 2;
        let msg_start = (wave_len - padded_msg_len) / 2;
        let msg_end = msg_start + padded_msg_len;

        for i in 0..wave_len {
            if i == pos as usize {
                let center = wave_len / 2;
                let dist = (pos - center as i32).abs();
                let max_dist = center as i32;
                let mut height = 7 - (dist * 7 / max_dist);
                if height < 0 { height = 0; }
                line.push_str(&format!("\x1b[1;97m{}\x1b[0m", wave_chars[height as usize]));
            } else if i >= msg_start && i < msg_end {
                let idx = i - msg_start;
                if idx == 0 || idx == padded_msg_len - 1 {
                    line.push(' ');
                } else {
                    line.push(message.chars().nth(idx - 1).unwrap_or(' '));
                }
            } else {
                line.push_str("\x1b[38;5;245m \x1b[0m");
            }
        }

        print!("\r\x1b[K{}", line);
        io::stdout().flush().unwrap();

        // Bounce effect
        pos += direction;
        if pos < 0 || pos >= wave_len as i32 {
            direction = -direction;
            pos += direction;
        }

        // Check if task is done
        if let Ok(result) = rx.try_recv() {
            print!("\r\x1b[K\x1b[?25h\n"); // clear line & show cursor
            io::stdout().flush().unwrap();
            return result;
        }

        // Animate speed based on position
        let center = wave_len as i32 / 2;
        let dist = (pos - center).abs();
        let max = center;
        let delay = match dist * 10 / max {
            9..=10 => 30,
            8 => 20,
            6..=7 => 10,
            4..=5 => 5,
            _ => 5,
        };

        sleep(Duration::from_millis(delay as u64)).await;
    }
}

/// Create an interactive selection menu
pub async fn create_menu(title: &str, items: &[String]) -> io::Result<usize> {
    loop {
        clear_screen()?;
        create_header(title);
        
        for (i, item) in items.iter().enumerate() {
            println!("{:2}. {}", i + 1, item);
        }
        
        println!();
        print!("{} ", "Select option (number):".bold());
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        match input.trim().parse::<usize>() {
            Ok(choice) if choice > 0 && choice <= items.len() => return Ok(choice - 1),
            _ => {
                print_error("Invalid selection. Please try again.");
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

/// Display a formatted list with status indicators
pub fn display_status_list(items: &[(String, bool)]) {
    for (item, status) in items {
        if *status {
            println!("{} {}", "●".green(), item);
        } else {
            println!("{} {}", "●".red().dimmed(), item.dimmed());
        }
    }
}

/// Create a code block display
pub fn display_code(code: &str) -> String {
    format!("{}", code.on_bright_black().white())
}

/// Create a beautiful notification block with rounded corners
pub fn create_notification(icon: &str, title: &str, message: &str, accent_color: u8) {
    let center_spaces = center_content(Some(80));
    let max_width = 70;
    
    // Split message into lines that fit the max width
    let mut lines = Vec::new();
    let mut current_line = String::new();
    
    for word in message.split_whitespace() {
        if current_line.len() + word.len() + 1 <= max_width - 8 { // Account for padding
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        } else {
            if !current_line.is_empty() {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    
    let content_width = lines.iter().map(|line| line.len()).max().unwrap_or(0).max(title.len() + 3);
    let block_width = content_width + 8; // 4 chars padding on each side
    
    // Top rounded border
    println!();
    println!("{}\x1b[38;5;{}m╭{}\x1b[0m", 
        center_spaces, accent_color, "─".repeat(block_width - 2));
    
    // Title line with icon
    let title_content = format!("{} {}", icon, title);
    let title_padding = block_width - title_content.len() - 4;
    let title_left_pad = title_padding / 2;
    let title_right_pad = title_padding - title_left_pad;
    
    println!("{}\x1b[38;5;{}m│\x1b[0m{}\x1b[1m{}\x1b[0m{}\x1b[38;5;{}m│\x1b[0m", 
        center_spaces, 
        accent_color, 
        " ".repeat(title_left_pad + 2),
        title_content,
        " ".repeat(title_right_pad + 2),
        accent_color
    );
    
    // Separator line
    println!("{}\x1b[38;5;{}m├{}\x1b[0m", 
        center_spaces, accent_color, "─".repeat(block_width - 2));
    
    // Message lines
    for line in &lines {
        let line_padding = block_width - line.len() - 4;
        println!("{}\x1b[38;5;{}m│\x1b[0m  {}{}\x1b[38;5;{}m│\x1b[0m", 
            center_spaces, 
            accent_color,
            line,
            " ".repeat(line_padding),
            accent_color
        );
    }
    
    // Bottom rounded border
    println!("{}\x1b[38;5;{}m╰{}\x1b[0m", 
        center_spaces, accent_color, "─".repeat(block_width - 2));
    println!();
}

/// Create an update notification specifically
pub fn create_update_notification(current_version: &str, latest_version: &str) {
    create_notification(
        "📦",
        "Update Available",
        &format!("A new version is available: {} → {}\nRun 'th update' to upgrade or 'th changelog' for details", 
            current_version, latest_version),
        33 // Orange/yellow color
    );
}

/// Print success message with green color
pub fn print_success(message: &str) {
    println!("✅ {}", message.green());
}

/// Print error message with red color  
pub fn print_error(message: &str) {
    println!("❌ {}", message.red());
}

/// Print info message with blue color
pub fn print_info(message: &str) {
    println!("ℹ️  {}", message.blue());
}

/// Print warning message with yellow color
pub fn print_warning(message: &str) {
    println!("⚠️  {}", message.yellow());
}

/// Get terminal width and calculate centering
pub fn center_content(content_width: Option<usize>) -> String {
    let term = Term::stdout();
    let term_width = term.size().1 as usize;
    let content_width = content_width.unwrap_or(65);
    let padding = if term_width > content_width { (term_width - content_width) / 2 } else { 0 };
    " ".repeat(padding)
}

/// Print centered text
pub fn cprintf(text: &str) {
    let center_spaces = center_content(None);
    print!("{}{}", center_spaces, text);
}

/// Print code block with styling
pub fn ccode(text: &str) -> String {
    format!("\x1b[38;5;245m▕\x1b[0m\x1b[48;5;245m{}\x1b[0m\x1b[38;5;245m▏\x1b[0m", text)
}

/// Create the original bash-style header
pub fn create_header_full(header_text: &str, center_spaces: &str, remove_new_line: bool) {
    let header_length = header_text.len();
    let total_dash_count: usize = 52;
    let available_dash_count = total_dash_count.saturating_sub(header_length.saturating_sub(5));
    let available_dash_count = if available_dash_count < 2 { 2 } else { available_dash_count };
    
    let left_dashes = available_dash_count / 2;
    let right_dashes = available_dash_count - left_dashes;
    
    let left_dash_str = "━".repeat(left_dashes);
    let right_dash_str = "━".repeat(right_dashes);
    
    if !remove_new_line { println!(); }
    println!("\x1b[0m\x1b[38;5;245m{}    ▄███████▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀███████████▀\x1b[0m\x1b[1;34m\x1b[0m", center_spaces);
    println!("\x1b[0m\x1b[38;5;245m{}  \x1b[0m\x1b[1m{} {}\x1b[0m\x1b[38;1m {} \x1b[0m\x1b[1;34m\x1b[0m", center_spaces, left_dash_str, header_text, right_dash_str);
    println!("\x1b[0m\x1b[38;5;245m{}▄███████████▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄███████▀\x1b[0m\x1b[1;34m\x1b[0m", center_spaces);
    println!();
}

pub fn create_header(header_text: &str) {
    let center_spaces = center_content(Some(1000));
    create_header_full(header_text, &center_spaces, false);
}

/// Print the original TH logo
pub fn print_logo(version: &str, center_spaces: &str) {
    println!();
    println!("{}                \x1b[0m\x1b[38;5;250m ▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁\x1b[0m\x1b[1;34m\x1b[0m", center_spaces);
    println!("{}                \x1b[0m\x1b[38;5;250m▕░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░▏\x1b[0m\x1b[1;34m\x1b[0m", center_spaces);
    println!("{}               \x1b[0m\x1b[38;5;250m▕░░░░░░░░░░░ \x1b[0m\x1b[1;97m████████╗ ██╗  ██╗\x1b[0m\x1b[38;5;250m ░░░░░░░░░░░░░▏\x1b[0m\x1b[1;34m\x1b[0m", center_spaces);
    println!("{}              \x1b[0m\x1b[38;5;249m▕▒▒▒▒▒▒▒▒▒▒▒ \x1b[0m\x1b[1;97m╚══██╔══╝ ██║  ██║\x1b[0m\x1b[38;5;249m ▒▒▒▒▒▒▒▒▒▒▒▒▒▏\x1b[0m\x1b[1;34m\x1b[0m", center_spaces);
    println!("{}             \x1b[0m\x1b[38;5;248m▕▓▓▓▓▓▓▓▓▓▓▓▓▓▓ \x1b[0m\x1b[1;97m█▉║    ███████║\x1b[0m\x1b[38;5;248m ▓▓▓▓▓▓▓▓▓▓▓▓▓▏\x1b[0m\x1b[1;34m\x1b[0m", center_spaces);
    println!("{}            \x1b[0m\x1b[38;5;247m▕██████████████ \x1b[0m\x1b[1;97m█▉║    ██╔══██║\x1b[0m\x1b[38;5;247m █████████████▏\x1b[0m\x1b[1;34m\x1b[0m", center_spaces);
    println!("{}           \x1b[0m\x1b[38;5;246m▕██████████████ \x1b[0m\x1b[1;97m██║    ██║  ██║\x1b[0m\x1b[38;5;246m █████████████▏\x1b[0m\x1b[1;34m\x1b[0m", center_spaces);
    println!("{}          \x1b[0m\x1b[38;5;245m▕██████████████ \x1b[0m\x1b[1;97m██╝    ██╝  ██╝\x1b[0m\x1b[38;5;245m █████████████▏\x1b[0m\x1b[1;34m\x1b[0m", center_spaces);
    println!("{}         \x1b[0m\x1b[38;5;245m▕██████████████▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄█████████████▏\x1b[0m\x1b[1;34m\x1b[0m", center_spaces);
    println!("{}         \x1b[0m\x1b[38;5;245m ▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔\x1b[0m\x1b[1;34m\x1b[0m", center_spaces);
    println!("{}         \x1b[0m\x1b[38;5;245m■■■■■■■■■\x1b[0m\x1b[1m Teleport Helper - v{} \x1b[0m\x1b[38;5;245m■■■■■■■■■■\x1b[0m\x1b[1;34m\x1b[0m", center_spaces, version);
    println!();
}

/// Print the original help screen (exactly like the bash version)
pub fn print_help(version: &str) {
    let center_spaces = center_content(None);

    print_logo(version, &center_spaces);
    create_header_full("Usage", &center_spaces, true);
    println!("{}     ╚═ \x1b[1mth aws  [options] | a\x1b[0m   : AWS login.", center_spaces);
    println!("{}     ╚═ \x1b[1mth db             | d\x1b[0m   : Log into our various databases.", center_spaces);
    println!("{}     ╚═ \x1b[1mth kube [options] | k\x1b[0m   : Kubernetes login.", center_spaces);
    println!("{}     ╚═ \x1b[1mth terra          | t\x1b[0m   : Quick log-in to Terragrunt.", center_spaces);
    println!("{}     ╚═ \x1b[1mth logout         | l\x1b[0m   : Clean up Teleport session.", center_spaces);
    println!("{}     ╚═ \x1b[1mth login          | li\x1b[0m  : Simple log in to Teleport\x1b[0m", center_spaces);
    println!("{}     \x1b[0m\x1b[38;5;245m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m\x1b[1;34m\x1b[0m", center_spaces);
    println!("{}     For help, and \x1b[1m[options]\x1b[0m info, run \x1b[1mth a/k/d etc.. -h\x1b[0m", center_spaces);
    println!();
    create_header_full("Docs", &center_spaces, true);
    println!("{}     Run the following commands to access the documentation pages: ", center_spaces);
    println!("{}     ╚═ \x1b[1mQuickstart:       | th qs\x1b[0m", center_spaces);
    println!("{}     ╚═ \x1b[1mDocs:             | th doc\x1b[0m", center_spaces);
    println!();
    create_header_full("Extras", &center_spaces, true);
    println!("{}     Run the following commands to access the extra features: ", center_spaces);
    println!("{}     ╚═ \x1b[1mth loader               \x1b[0m: Run loader animation.", center_spaces);
    println!("{}     ╚═ \x1b[1mth animate [options]    \x1b[0m: Run logo animation.", center_spaces);
    println!("{}        ╚═ \x1b[1myl", center_spaces);
    println!("{}        ╚═ \x1b[1mth", center_spaces);
    println!("{}          \x1b[0m\x1b[38;5;245m  ▁▁▁▁▁▁▁▁▁▁▁▁▁\x1b[0m\x1b[1;97m  ▄▄▄ ▄▁▄  \x1b[0m\x1b[38;5;245m▁▁▁▁▁▁▁▁▁▁▁▁▁▁\x1b[0m\x1b[1;34m\x1b[0m", center_spaces);
    println!("{}          \x1b[0m\x1b[38;5;245m ▔▔▔▔▔▔▔▔▔▔▔▔▔▔\x1b[0m\x1b[1;97m   ▀  ▀▔▀  \x1b[0m\x1b[38;5;245m▔▔▔▔▔▔▔▔▔▔▔▔▔\x1b[0m\x1b[1;34m\x1b[0m", center_spaces);
}

// ========================================================================================================================
//                                                   Exact Bash Functions 
// ========================================================================================================================

/// Find available port - exactly like bash version
pub fn find_available_port() -> u16 {
    let mut rng = rand::thread_rng();
    for _ in 0..100 {
        let port = rng.gen_range(40000..60000);
        if !is_port_in_use(port) {
            return port;
        }
    }
    50000
}

fn is_port_in_use(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_err()
}

/// Create a note - exactly like bash version
pub fn create_note(note_text: &str) {
    println!("\n\n\x1b[0m\x1b[38;5;245m▄██▀ {}\x1b[0m\x1b[1;34m\x1b[0m\n\n", note_text);
}

/// Demo wave loader - exactly like bash version
pub async fn demo_wave_loader(message: Option<&str>) {
    let message = message.unwrap_or("Demo Wave Loader");
    
    clear_screen().unwrap();
    
    println!("\nPress Ctrl+C to exit (Spam it, if it doesn't work first time!)\n\n");
    
    // Create a long-running background task
    let _handle = tokio::spawn(async {
        tokio::time::sleep(Duration::from_secs(9999)).await;
    });
    
    // Run the wave loader indefinitely
    load_content(message, async {
        tokio::time::sleep(Duration::from_secs(9999)).await;
        ()
    }).await;
}

/// Animate the TH logo - exactly like bash version  
pub async fn animate_th() {
    let center_spaces = center_content(None);
    
    clear_screen().unwrap();
    print!("\x1b[?25l"); // Hide cursor
    
    println!("\n\x1b[1mTeleport Helper - Press Enter to continue...\x1b[0m\n\n");
    
    let colors = [232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255, 254, 253, 252, 251, 250, 249, 248, 247, 246, 245, 244, 243, 242, 241, 240, 239, 238, 237, 236, 235, 234, 233];
    let mut frame = 0;
    
    print!("\x1b[s\x1b[2J\x1b[H"); // Save cursor, clear screen, go home
    
    loop {
        print!("\x1b[H"); // Move to home
        
        let line1_color = colors[(frame + 0) % colors.len()];
        let line2_color = colors[(frame + 1) % colors.len()];
        let line3_color = colors[(frame + 2) % colors.len()];
        let line4_color = colors[(frame + 3) % colors.len()];
        let line5_color = colors[(frame + 4) % colors.len()];
        let line6_color = colors[(frame + 5) % colors.len()];
        let line7_color = colors[(frame + 6) % colors.len()];
        let line8_color = colors[(frame + 7) % colors.len()];
        let line9_color = colors[(frame + 8) % colors.len()];
        let line10_color = colors[(frame + 9) % colors.len()];
        let line11_color = colors[(frame + 10) % colors.len()];
        
        println!("{}        \x1b[38;5;{}m ▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁\x1b[0m", center_spaces, line11_color);
        println!("{}        \x1b[38;5;{}m▕░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░▏\x1b[0m", center_spaces, line10_color);
        println!("{}       \x1b[38;5;{}m▕░░░░░░░░░░░░░░░░░ \x1b[1;97m███████████╗ ███╗  ███╗\x1b[38;5;{}m ░░░░░░░░░░░░░░░░░░░░▏\x1b[0m", center_spaces, line9_color, line9_color);
        println!("{}      \x1b[38;5;{}m▕▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒ \x1b[1;97m╚══███╔══╝  ███║  ███║\x1b[38;5;{}m ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▏\x1b[0m", center_spaces, line8_color, line8_color);
        println!("{}     \x1b[38;5;{}m▕▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ \x1b[1;97m███║     █████████║\x1b[38;5;{}m ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▏\x1b[0m", center_spaces, line7_color, line7_color);
        println!("{}    \x1b[38;5;{}m▕█████████████████████ \x1b[1;97m███║     ███╔══███║\x1b[38;5;{}m ████████████████████▏\x1b[0m", center_spaces, line6_color, line6_color);
        println!("{}   \x1b[38;5;{}m▕█████████████████████ \x1b[1;97m███║     ███║  ███║\x1b[38;5;{}m ████████████████████▏\x1b[0m", center_spaces, line5_color, line5_color);
        println!("{}  \x1b[38;5;{}m▕█████████████████████ \x1b[1;97m███╝     ███╝  ███╝\x1b[38;5;{}m ████████████████████▏\x1b[0m", center_spaces, line4_color, line4_color);
        println!("{} \x1b[38;5;{}m▕█████████████████████ \x1b[1;97m███╝     ███╝  ███╝\x1b[38;5;{}m ████████████████████▏\x1b[0m", center_spaces, line4_color, line4_color);
        println!("{}\x1b[38;5;{}m▕█████████████████████▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄████████████████████▏\x1b[0m", center_spaces, line3_color);
        println!("{}\x1b[38;5;{}m ▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔\x1b[0m", center_spaces, line2_color);
        println!();
        
        frame += 1;
        sleep(Duration::from_millis(80)).await;
    }
}

/// Animate the Youlend logo - exactly like bash version 
pub async fn animate_youlend() {
    let center_spaces = center_content(Some(92));
    
    clear_screen().unwrap();
    print!("\x1b[?25l"); // Hide cursor
    
    let colors = [232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255, 254, 253, 252, 251, 250, 249, 248, 247, 246, 245, 244, 243, 242, 241, 240, 239, 238, 237, 236, 235, 234, 233];
    let mut frame = 0;
    
    print!("\x1b[s\x1b[2J\x1b[H"); // Save cursor, clear screen, go home
    
    loop {
        print!("\x1b[H"); // Move to home
        
        let line1_color = colors[(frame + 0) % colors.len()];
        let line2_color = colors[(frame + 1) % colors.len()];
        let line3_color = colors[(frame + 2) % colors.len()];
        let line4_color = colors[(frame + 3) % colors.len()];
        let line5_color = colors[(frame + 4) % colors.len()];
        let line6_color = colors[(frame + 5) % colors.len()];
        let line7_color = colors[(frame + 6) % colors.len()];
        let line8_color = colors[(frame + 7) % colors.len()];
        let line9_color = colors[(frame + 8) % colors.len()];
        
        println!("{}       \x1b[38;5;{}m ▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁\x1b[0m", center_spaces, line9_color);
        println!("{}       \x1b[38;5;{}m▕░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░▏\x1b[0m", center_spaces, line8_color);
        println!("{}      \x1b[38;5;{}m▕░░░░░░░░░░ \x1b[1;97m██╗   ██╗ ██████╗  ██╗   ██╗ ██╗      ███████╗ ███╗   ██╗ ██████╗\x1b[38;5;{}m  ░░░░░░░░▏\x1b[0m", center_spaces, line7_color, line7_color);
        println!("{}     \x1b[38;5;{}m▕▒▒▒▒▒▒▒▒▒▒ \x1b[1;97m╚██╗ ██╔╝██╔═══██╗ ██║   ██║ ██║      ██╔════╝ ████╗  ██║ ██╔══██╗\x1b[38;5;{}m ▒▒▒▒▒▒▒▒▏\x1b[0m", center_spaces, line6_color, line6_color);
        println!("{}    \x1b[38;5;{}m▕▓▓▓▓▓▓▓▓▓▓ \x1b[1;97m ╚████╔╝ ██║   ██║ ██║   ██║ ██║      █████╗   ██╔██╗ ██║ ██║  ██║\x1b[38;5;{}m ▓▓▓▓▓▓▓▓▏\x1b[0m", center_spaces, line5_color, line5_color);
        println!("{}   \x1b[38;5;{}m▕██████████ \x1b[1;97m  ╚██╔╝  ██║   ██║ ██║   ██║ ██║      ██╔══╝   ██║╚██╗██║ ██║  ██║\x1b[38;5;{}m ████████▏\x1b[0m", center_spaces, line4_color, line4_color);
        println!("{}  \x1b[38;5;{}m▕██████████ \x1b[1;97m   ██║   ╚██████╔╝ ╚██████╔╝ ███████╗ ███████╗ ██║ ╚████║ ██████╔╝\x1b[38;5;{}m ████████▏\x1b[0m", center_spaces, line3_color, line3_color);
        println!("{} \x1b[38;5;{}m ██████████ \x1b[1;97m   ╚═╝    ╚═════╝   ╚═════╝  ╚══════╝ ╚══════╝ ╚═╝  ╚═══╝ ╚═════╝\x1b[38;5;{}m  ████████▏\x1b[0m", center_spaces, line2_color, line2_color);
        println!("{}\x1b[38;5;{}m▕██████████▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄████████▏\x1b[0m", center_spaces, line1_color);
        println!("{}\x1b[38;5;{}m ▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔\x1b[0m", center_spaces, line1_color);
        println!();

        frame += 1;
        sleep(Duration::from_millis(80)).await;
    }
}

// ========================================================================================================================
//                                                    Core Helper Functions
// ========================================================================================================================

/// Login to Teleport - exactly like bash th_login function
pub async fn th_login() -> anyhow::Result<()> {
    use std::process::Command;
    
    clear_screen()?;
    create_header("Login");
    println!("Checking login status...");
    
    // Check if already logged in - exactly like bash: tsh status 2>/dev/null | grep -q 'Logged in as:'
    let status_check = Command::new("tsh")
        .args(["status"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output();
        
    if let Ok(output) = status_check {
        let output_str = String::from_utf8_lossy(&output.stdout);
        if output_str.contains("Logged in as:") {
            cprintf("\n✅ \x1b[1mAlready logged in to Teleport!\x1b[0m\n");
            std::thread::sleep(std::time::Duration::from_secs(1));
            return Ok(());
        }
    }
    
    println!("\nLogging you into Teleport...");
    
    // Run tsh login - exactly like bash: tsh login --auth=ad --proxy=youlend.teleport.sh:443 > /dev/null 2>&1
    let _login_result = Command::new("tsh")
        .args(["login", "--auth=ad", "--proxy=youlend.teleport.sh:443"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    
    // Wait until login completes (max 15 seconds) - exactly like bash: for i in {1..30}
    for _ in 1..=30 {
        let status_check = Command::new("tsh")
            .args(["status"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .output();
            
        if let Ok(output) = status_check {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if output_str.contains("Logged in as:") {
                println!("\n\x1b[1;32mLogged in successfully!\x1b[0m");
                std::thread::sleep(std::time::Duration::from_secs(1));
                return Ok(());
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    
    println!("\n❌ \x1b[1;31mTimed out waiting for Teleport login.\x1b[0m");
    Err(anyhow::anyhow!("Login timeout"))
}

/// Kill/cleanup Teleport sessions - exactly like bash th_kill function
pub async fn th_kill() -> anyhow::Result<()> {
    use std::process::Command;
    use std::fs;
    use std::path::Path;
    
    clear_screen()?;
    create_header("Cleanup");
    println!("🧹 \x1b[1mCleaning up Teleport session...\x1b[0m");
    
    // Remove temp credential files - exactly like bash with glob patterns
    let temp_dir = std::path::Path::new("/tmp");
    if let Ok(entries) = fs::read_dir(temp_dir) {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();
            if name.starts_with("yl") || name.starts_with("tsh") || name.starts_with("admin_") {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
    
    // Determine which shell profile to clean - exactly like bash
    let shell = std::env::var("SHELL").unwrap_or_default();
    let shell_name = Path::new(&shell).file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    
    let shell_profile = match shell_name {
        "zsh" => Some(format!("{}/.zshrc", std::env::var("HOME").unwrap_or_default())),
        "bash" => Some(format!("{}/.bash_profile", std::env::var("HOME").unwrap_or_default())),
        _ => {
            println!("Unsupported shell: {}. Skipping profile cleanup.", shell_name);
            None
        }
    };
    
    // Remove any lines sourcing proxy envs from the profile - exactly like bash sed
    if let Some(profile_path) = &shell_profile {
        if Path::new(profile_path).exists() {
            if let Ok(content) = fs::read_to_string(profile_path) {
                // Filter out lines that match the sed pattern: '/[[:space:]]*source \/tmp\/tsh_proxy_/d'
                let cleaned_lines: Vec<&str> = content
                    .lines()
                    .filter(|line| {
                        let trimmed = line.trim_start();
                        !trimmed.starts_with("source /tmp/tsh_proxy_")
                    })
                    .collect();
                
                let _ = fs::write(profile_path, cleaned_lines.join("\n"));
                println!("\n\n✏️ \x1b[0mRemoving source lines from {}...\x1b[0m", profile_path);
            }
        }
    }
    
    println!("\n📃 \x1b[0mRemoving ENVs...\x1b[0m");
    
    // Unset environment variables - exactly like bash
    std::env::remove_var("AWS_ACCESS_KEY_ID");
    std::env::remove_var("AWS_SECRET_ACCESS_KEY");
    std::env::remove_var("AWS_CA_BUNDLE");
    std::env::remove_var("HTTPS_PROXY");
    std::env::remove_var("ACCOUNT");
    std::env::remove_var("ROLE");
    std::env::remove_var("AWS_DEFAULT_REGION");
    
    println!("\n💀 \x1b[0mKilling all running tsh proxies...\x1b[0m\n");
    
    // Kill all tsh proxy processes - exactly like bash: ps aux | grep '[t]sh proxy aws' | awk '{print $2}' | xargs kill
    let ps_output = Command::new("ps")
        .args(["aux"])
        .output();
        
    if let Ok(output) = ps_output {
        let ps_lines = String::from_utf8_lossy(&output.stdout);
        let mut pids_to_kill = Vec::new();
        
        // Find tsh proxy aws processes (excluding grep itself)
        for line in ps_lines.lines() {
            if line.contains("tsh proxy aws") && !line.contains("grep") {
                if let Some(pid_str) = line.split_whitespace().nth(1) {
                    pids_to_kill.push(pid_str.to_string());
                }
            }
        }
        
        // Find tsh proxy db processes (excluding grep itself)
        for line in ps_lines.lines() {
            if line.contains("tsh proxy db") && !line.contains("grep") {
                if let Some(pid_str) = line.split_whitespace().nth(1) {
                    pids_to_kill.push(pid_str.to_string());
                }
            }
        }
        
        // Kill all found PIDs
        if !pids_to_kill.is_empty() {
            let _ = Command::new("kill")
                .args(pids_to_kill)
                .output();
        }
    }
    
    // Logout from teleport - exactly like bash
    let _ = Command::new("tsh")
        .args(["logout"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output();
        
    let _ = Command::new("tsh")
        .args(["apps", "logout"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output();
    
    println!("\n✅ \x1b[1;32mLogged out of all apps, clusters & proxies\x1b[0m\n");
    
    Ok(())
}