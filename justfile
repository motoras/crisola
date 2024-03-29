set positional-arguments

alias b := build
alias c := clean
alias r := release
alias t := test
alias u := upg

#Displays all the available commands
default:
  @just --list

#Upgrade cargo dependencies
upg:
  cargo upgrade

#Build in debug mode
build:
  cargo build

#Run tests
test:
  cargo build

#Build in release mode
release:
  cargo build --release
  
#Clean the build artifacts
clean:
  cargo clean
