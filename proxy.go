package main

import (
  "bufio"
  "flag"
  "fmt"
  "os"
  "os/exec"
  "sync"
)

func main() {
  // Parse flags: -log for log filename
  logPath := flag.String("log", "wrapper.log", "path to log file")
  flag.Parse()
  if flag.NArg() < 1 {
    fmt.Fprintf(os.Stderr, "usage: %s [ -log path ] command [args...]\n",
      os.Args[0])
    os.Exit(1)
  }

  // Open log file
  lf, err := os.OpenFile(*logPath, os.O_CREATE|os.O_WRONLY, 0644)
  if err != nil {
    fmt.Fprintf(os.Stderr, "open log: %v\n", err)
    os.Exit(1)
  }
  defer lf.Close()

  // Prepare command
  cmd := exec.Command(flag.Arg(0), flag.Args()[1:]...)
  stdinPipe, err := cmd.StdinPipe()
  if err != nil {
    fmt.Fprintf(os.Stderr, "stdin pipe: %v\n", err)
    os.Exit(1)
  }
  stdoutPipe, err := cmd.StdoutPipe()
  if err != nil {
    fmt.Fprintf(os.Stderr, "stdout pipe: %v\n", err)
    os.Exit(1)
  }
  cmd.Stderr = os.Stderr // pass through stderr unlogged

  if err := cmd.Start(); err != nil {
    fmt.Fprintf(os.Stderr, "start cmd: %v\n", err)
    os.Exit(1)
  }

  var wg sync.WaitGroup
  wg.Add(2)

  // stdin → child stdin, logging
  go func() {
    defer wg.Done()
    r := bufio.NewReader(os.Stdin)
    buf := make([]byte, 4096)
    for {
      n, err := r.Read(buf)
      if n > 0 {
        // write to log with prefix
        lf.Write([]byte("[IN] "))
        lf.Write(buf[:n])
        lf.Write([]byte("\n"))
        stdinPipe.Write(buf[:n])
      }
      if err != nil {
        stdinPipe.Close()
        return
      }
    }
  }()

  // child stdout → stdout, logging
  go func() {
    defer wg.Done()
    r := bufio.NewReader(stdoutPipe)
    buf := make([]byte, 4096)
    for {
      n, err := r.Read(buf)
      if n > 0 {
        lf.Write([]byte("[OUT] "))
        lf.Write(buf[:n])
        lf.Write([]byte("\n"))
        os.Stdout.Write(buf[:n])
      }
      if err != nil {
        return
      }
    }
  }()

  wg.Wait()
  cmd.Wait()
}
