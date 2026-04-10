# To use: brew tap DaviRain-Su/gradience
#         brew install gradience
class Gradience < Formula
  desc "Agent Wallet Orchestration Platform"
  homepage "https://gradience-wallet.vercel.app"
  version "0.1.3"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/DaviRain-Su/gradience-wallet/releases/download/v0.1.3/gradience-aarch64-apple-darwin.tar.gz"
      sha256 "176a2ba3c3ff7e57d432c57962ac4694b9b5def8646b4bc8df44e6895978d15b"
    else
      odie "Intel Mac is not supported in pre-built binaries. Please build from source with: cargo install --path crates/gradience-cli"
    end
  end

  on_linux do
    url "https://github.com/DaviRain-Su/gradience-wallet/releases/download/v0.1.3/gradience-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "b29e03dfe7ae1c412cd38dac0268a67e3474abd41c9d50c3771be46454ec3092"
  end

  def install
    bin.install "gradience"
  end

  test do
    system "#{bin}/gradience", "--help"
  end
end
