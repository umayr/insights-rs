# `insights`
> process exported whatsapp chat and generates insights.

# Usage

You need to have rust and cargo set up on your machine as it's not published anywhere

```b
λ git clone github.com/umayr/insights-rs
λ cd insights-rs
 
λ cargo build --release

λ # in case you find it interesting enough
λ mv target/release/insights /usr/local/bin
```

Once you have that out of the way, you can use it like:
```
λ insights -h

Insights - A minimalistic whatsapp chat analyser.

Usage:
    insights <file> [--pretty] [--timeline=<duration>]
    insights (-h | --help)
    insights --version

Options:
    -h --help                   shows this usage
    --version                   shows the version of application
    --pretty                    prints the analysis in pretty format
    --timeline=<duration>       sets the duration of the timeline [default: monthly]
                                options:
                                    - daily
                                    - weekly
                                    - monthly
                                    - yearly

λ insights path/to/exported/chat/file.txt
# [...]
```

