# To use: brew tap DaviRain-Su/gradience
#         brew install gradience
class Gradience < Formula
  desc "Agent Wallet Orchestration Platform"
  homepage "https://gradience-wallet.vercel.app"
  version "0.1.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/DaviRain-Su/gradience-wallet/releases/download/v0.1.0/gradience-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_ARM64"
    else
      url "https://github.com/DaviRain-Su/gradience-wallet/releases/download/v0.1.0/gradience-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_X86_64"
    end
  end

  on_linux do
    url "https://github.com/DaviRain-Su/gradience-wallet/releases/download/v0.1.0/gradience-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "PLACEHOLDER_SHA256_LINUX"
  end

  def install
    bin.install "gradience"
  end

  test do
    system "#{bin}/gradience", "--help"
  end
end
