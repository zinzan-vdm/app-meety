# Homebrew cask for Meety — local-first meeting transcription for macOS.
#
# This repository doubles as its own Homebrew tap. Install with:
#
#   brew tap woosal1337/meety https://github.com/woosal1337/meety
#   brew install --cask meety
#
# The `version` and `sha256` lines below are updated automatically by the
# `update-homebrew-cask` job in .github/workflows/release.yml on every tagged
# release, so `brew upgrade --cask meety` tracks new releases.
cask "meety" do
  version "2.0.0"
  sha256 "eea15330793681d57be1301265438686e63173081bcbec45580ca8c1ab553ea9"

  url "https://github.com/woosal1337/meety/releases/download/v#{version}/Meety_#{version}_aarch64.dmg",
      verified: "github.com/woosal1337/meety/"
  name "Meety"
  desc "Local-first meeting transcription"
  homepage "https://github.com/woosal1337/meety"

  livecheck do
    url :url
    strategy :github_latest
  end

  depends_on arch: :arm64
  depends_on macos: :ventura

  app "Meety.app"

  zap trash: [
    "~/Library/Application Support/Meety",
    "~/Library/Caches/dev.meety.app",
    "~/Library/HTTPStorages/dev.meety.app",
    "~/Library/Preferences/dev.meety.app.plist",
    "~/Library/Saved Application State/dev.meety.app.savedState",
  ]
end
