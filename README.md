# TextFix

This program will fix the text that you select using OpenAI.

*This will use the clipboard for that, so you will lose the content of your clipboard (sorry)*

## Configuration

The API key should be inside your environment as `OPENAI_API_KEY`.

You can also create a `.env` file at the root of this repository.

## Usage

Compile and run the program in a terminal

```bash
cargo run
```

Then anywhere in your OS, you can select editable text and press **CAPSLOCK** to replace the content with the fixed content.

To exit the program, you can either exit the terminal or press `CTRL+C` inside the terminal.
