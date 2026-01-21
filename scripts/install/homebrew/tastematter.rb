# typed: false
# frozen_string_literal: true

# Homebrew formula for tastematter CLI
# Install: brew tap jacob-dietle/tastematter && brew install tastematter
class Tastematter < Formula
  desc "Context intelligence CLI for Claude Code power users"
  homepage "https://github.com/jacob-dietle/tastematter"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_intel do
      url "https://install.tastematter.dev/releases/v0.1.0/tastematter-darwin-x86_64"
      sha256 ""
    end
    on_arm do
      url "https://install.tastematter.dev/releases/v0.1.0/tastematter-darwin-aarch64"
      sha256 ""
    end
  end

  def install
    binary_name = "tastematter"
    if Hardware::CPU.intel?
      bin.install "tastematter-darwin-x86_64" => binary_name
    else
      bin.install "tastematter-darwin-aarch64" => binary_name
    end
  end

  test do
    assert_match "tastematter", shell_output("#{bin}/tastematter --version")
  end
end
