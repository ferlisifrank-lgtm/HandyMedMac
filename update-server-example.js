// Example update server using Express
// Deploy this to any Node.js hosting (Vercel, Railway, etc.)

const express = require("express");
const app = express();

// Serve latest.json dynamically
app.get("/handy/latest.json", (req, res) => {
  const latestVersion = {
    version: "0.6.9",
    notes: "- Added setup wizard\n- Improved privacy settings\n- Bug fixes",
    pub_date: new Date().toISOString(),
    platforms: {
      "darwin-aarch64": {
        signature: "YOUR_SIGNATURE_HERE",
        url: "https://yourdomain.com/releases/Handy_0.6.9_aarch64.app.tar.gz",
      },
      "darwin-x86_64": {
        signature: "YOUR_SIGNATURE_HERE",
        url: "https://yourdomain.com/releases/Handy_0.6.9_x64.app.tar.gz",
      },
    },
  };

  res.json(latestVersion);
});

// Health check
app.get("/health", (req, res) => {
  res.json({ status: "ok" });
});

const PORT = process.env.PORT || 3000;
app.listen(PORT, () => {
  console.log(`Update server running on port ${PORT}`);
});
