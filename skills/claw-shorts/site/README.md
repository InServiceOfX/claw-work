# claw-shorts site

Static site with free tools for making launch videos for dev tools and indie apps.

## Contents

- **Launch Shorts Planner**: Generate a 5-Short launch plan from a README
- **Safe Zone Checker**: Visualize caption danger zones for Shorts/TikTok/Reels
- **Command Builder**: Generate ffmpeg and agg commands for video conversion
- 3 guide pages with embedded tools

## Preview locally

Open `index.html` directly in a browser:

```bash
# From this directory
open index.html

# Or use a local server (optional, for relative paths)
python3 -m http.server 8000
# Then visit http://localhost:8000
```

The site works entirely offline with no external dependencies.

## Run tests

```bash
node tests/test_planner.js
```

Tests validate the planner's JSON output structure against the expected schema.

## Structure

```
site/
├── index.html              # Home page
├── css/style.css           # Shared styles
├── js/
│   ├── planner.js          # Launch Shorts Planner logic
│   ├── safe-zones.js       # Safe Zone Checker logic
│   └── command-builder.js  # Command Builder logic
├── tools/
│   ├── planner.html        # Planner tool page
│   ├── safe-zones.html     # Safe Zone Checker tool page
│   └── command-builder.html # Command Builder tool page
├── pages/
│   ├── launch-video.html   # Guide: How to make a launch video
│   ├── safe-zones.html     # Guide: Safe zones for captions
│   └── terminal-to-short.html # Guide: Terminal to vertical MP4
├── tests/
│   └── test_planner.js     # Tests for planner JSON output
├── sitemap.xml             # Sitemap (placeholder base URL)
├── robots.txt              # Robots (placeholder base URL)
└── README.md               # This file
```

## Configuration

Before deploying, replace `PLACEHOLDER_BASE_URL` in these files with your actual base URL:

- `sitemap.xml`
- `robots.txt`
- Schema.org `url` fields in HTML files

## License

MIT License. See repository LICENSE file.

## Related

- [claw_shorts.py](../scripts/claw_shorts.py) - Python CLI for rendering videos
- [claw-shorts SKILL.md](../skills/claw-shorts/SKILL.md) - Agent skill for video production
- [claw-work repository](https://github.com/InServiceOfX/claw-work)
