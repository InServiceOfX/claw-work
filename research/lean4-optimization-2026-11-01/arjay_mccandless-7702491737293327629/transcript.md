# Transcript: Lean vs $12 server?!  #coding #programming

Source: <https://www.tiktok.com/@arjay_mccandless/video/7702491737293327629?_r=1&_t=ZP-9B32vwwlDyw>  
Engine: faster-whisper `small`. Machine transcribed; correct obvious mishearings in README.md.

- `[0:00.0 - 0:04.2]` Somebody used mathematical proofs to build the most efficient web server for my $12 server
- `[0:04.2 - 0:05.9]` challenge and the results are insane.
- `[0:05.9 - 0:09.8]` For context, a couple weeks ago I challenged you to build a small social media API that
- `[0:09.8 - 0:14.2]` could support as many users as possible on a tiny server with one vcpu and two gigs
- `[0:14.2 - 0:15.2]` of RAM.
- `[0:15.2 - 0:18.3]` And for a while now, C, Rust, and Assembly have been fighting for the top spots.
- `[0:18.3 - 0:20.9]` Instead of writing their server in one of those languages directly, this person
- `[0:20.9 - 0:22.4]` started with Lean 4.
- `[0:22.4 - 0:25.4]` Lean is a programming language that lets you write mathematical proofs and have
- `[0:25.4 - 0:27.2]` a computer verify that they're correct.
- `[0:27.2 - 0:30.2]` If it sounds familiar, it's probably because it's been coming a lot recently with all
- `[0:30.2 - 0:32.4]` the new mathematical research major labs are doing.
- `[0:32.4 - 0:36.0]` Anyways, they used it to describe how the application should work, prove certain properties
- `[0:36.0 - 0:39.6]` about the data, and then use those proofs to generate optimized C code.
- `[0:39.6 - 0:40.6]` Here's why this is useful.
- `[0:40.6 - 0:44.0]` Normally, when somebody loads the feed, the database has to find the newest posts,
- `[0:44.0 - 0:45.9]` retrieve those posts, and count their likes.
- `[0:45.9 - 0:49.4]` This developer proved that as long as certain conditions hold, those posts are already
- `[0:49.4 - 0:51.2]` stored in the exact order they need.
- `[0:51.2 - 0:54.8]` So instead of doing any extra work, they can just read the most recent posts directly
- `[0:54.8 - 0:56.9]` from SQLite's underlying B tree.
- `[0:56.9 - 1:00.9]` They also bypassed SQLite's normal query execution engine and read directly from the
- `[1:00.9 - 1:02.8]` B tree using its B tree API.
- `[1:02.8 - 1:07.9]` This reduced time to load the feed from 16.7 microseconds all the way down to 5.4, roughly
- `[1:07.9 - 1:08.9]` three times faster.
- `[1:08.9 - 1:12.7]` Technically, you could write these exact optimizations in C yourself without ever touching
- `[1:12.7 - 1:13.7]` Lean.
- `[1:13.7 - 1:17.2]` Lean just lets you prove that these optimizations are safe for your specific workload.
- `[1:17.2 - 1:20.5]` They also made some pretty clever optimizations with IOU ring, which basically all of
- `[1:20.5 - 1:21.8]` our top submissions did.
- `[1:21.8 - 1:27.2]` When I benchmarked it, this submission supported over 280,000 concurrent users or 26,000 requests
- `[1:27.2 - 1:27.6]` per second.
