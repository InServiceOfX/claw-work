# Harness-neutral social publishing: feasibility

Research checked 2026-11-06 against the official pages linked below. This is a technical feasibility assessment, not platform approval or a legal opinion. No account login, OAuth consent, developer-app creation, upload, or publication was performed.

## Recommendation

1. Build a small local YouTube CLI using Google's official API. A shell-capable harness can invoke the same program; no harness-specific plugin is required.
2. Do not build a private-use TikTok Direct Post API client for account(s) managed by the operator. The published intended-use guidelines expressly exclude this use case.
3. For TikTok now, use the normal signed-in posting interface manually, or evaluate an existing approved publisher. Human-supervised browser assistance may reduce clicks, but is not an official publishing API and is not a guarantee of platform-policy compliance. Investigate applicable terms before automation; do not bypass authentication challenges or anti-abuse controls.
4. Consider TikTok's official inbox upload only if app/scope approval is available and a human finishing the post in TikTok is useful. It is not unattended publication.

## YouTube: practical first integration

The official API supports media upload, metadata, private/public visibility, channel identity reads and processing-status reads. Installed desktop OAuth with a loopback callback keeps sign-in on Google's own consent page. A user still must enable the API, configure the OAuth app, download a Desktop client configuration and authorize the tool.

A reusable local CLI is simpler than five integrations: Pi, Hermes Agent, Claude Code, Codex and Grok Build all need only shell access to the same isolated environment. Tokens stay in protected local storage; they must never be passed through prompts or shell flags.

### Important correction to older advice

On the check date, `videos.insert` explicitly states:

> Videos uploaded from unverified API projects are not restricted to private viewing mode.

The same page says an omitted `status.privacyStatus` defaults to **public**, subject to creator account settings. The tool therefore must explicitly set **private**, not rely on platform defaults.

Older descriptions of a blanket post-2020 private-only upload/audit restriction must not be repeated as current fact. This does not promise that a particular account, OAuth client or video is publishable; consent, account restrictions, policy compliance and video rights remain separate. OAuth app verification, YouTube API compliance audits and video copyright checks are distinct topics. Additional quota can require a compliance audit. Recheck the live docs if behavior or policy differs.

### Trade-offs

Advantages: structured results, stable IDs, correct-account checks, status readback, local receipts and independent approval gates. No browser selector maintenance.

Costs: one-time Cloud/OAuth setup, token lifecycle maintenance and potentially verification/review depending on distribution/scopes. A project left in OAuth Testing can require frequent reauthorization (Google documents seven-day refresh-token expiry for relevant external Testing apps; exceptions apply). `youtube.force-ssl` grants more than upload alone because later publication uses update operations; explain that permission rather than presenting it as read-only.

A successful API upload does not prove copyright clearance, unrestricted visibility, Shorts classification, monetization eligibility, or reach. The Data API does not supply a universal copyright-clearance result. Manually review Studio checks before approving public publication.

### Sources

- Upload and privacy defaults: https://developers.google.com/youtube/v3/docs/videos/insert
- Desktop OAuth: https://developers.google.com/youtube/v3/guides/auth/installed-apps
- Audits and quota: https://developers.google.com/youtube/v3/guides/quota_and_compliance_audits
- Refresh token lifecycle: https://developers.google.com/identity/protocols/oauth2#expiration

## TikTok: official API is not a good private Direct Post fit

### Direct Post (`video.publish`)

The official getting-started guide requires a registered app, Content Posting API/Direct Post configuration, scope approval, creator authorization and an audit to lift unaudited visibility restrictions.

The Content Sharing Guidelines explicitly say:

> API Clients must not be limited to test applications and should be intended for a wide audience, not limited to internal groups/private use.
>
> Not acceptable: A utility tool to help upload contents to the account(s) you or your team manages.

That is a product-fit barrier, not merely an engineering task or a reason to disguise the app as a public service. A public source repository alone does not make the actual private-use product eligible.

Unaudited Direct Post clients can allow up to five users per 24-hour window, and the guidelines say those accounts must be private when posting; content must use `SELF_ONLY`. Do not turn an established public account private just to test an integration. Audited clients still have posting/creator caps.

An approved integration also needs a real user-controlled publishing UX: current creator identity, duration limit, user-selected privacy (no preset default), interaction controls consistent with creator settings, editable caption/hashtags, preview, explicit upload consent, music-use confirmation and commercial-content disclosures where appropriate. A headless agent choosing everything is not an equivalent implementation.

The guidelines also restrict promotional/watermark material added by integrations. Branded promo exports need careful review; do not assume a video containing promotional links is automatically appropriate for this API. This assessment does not determine the moderation outcome of a manual post.

**Verdict:** not recommended as a custom personal publisher. If building a genuinely public creator product later, reassess with truthful app details and its actual audience/UX.

### Inbox upload (`video.upload`)

The official Upload API sends a video to TikTok without posting it. TikTok explicitly instructs clients to tell users to click their inbox notification, continue editing and complete the post in TikTok.

Benefits: official media-transfer path; human final control. Local files can use `FILE_UPLOAD`, avoiding a public media host and domain verification required for `PULL_FROM_URL`.

Costs: registered app, appropriate product/scope approval, OAuth, API and pending-share limits, and a manual app handoff. The reference documents at most five pending shares within 24 hours. Approval for this particular private-use app was not established; Direct Post's intended-use text should not be stretched into a promise that inbox upload is approved.

**Verdict:** possible investigation milestone, not the first implementation and not an unattended publication solution.

### Normal browser posting

Benefits: uses the familiar creator interface; can expose cover selection, music/rights warnings and disclosure controls in their native context. No custom developer-app approval is needed for ordinary manual use.

Costs: automation is brittle, UI/localization changes break selectors, account selection needs explicit checks, login/CAPTCHA/2FA require the user, and platform terms/anti-automation rules still apply. A signed-in browser is not automatically controllable from an agent.

If pursuing assistance: use a dedicated local browser profile with human login, narrow browser-control access, no exported cookies/passwords and a visible preview. Stop at authentication challenges or unexpected dialogs. Require explicit confirmation for the exact account, file, caption, visibility and final submission. Confirm success by actual post URL/status, not a click alone. No implementation or policy-compliance claim is made here.

### Existing approved publisher

Potentially less engineering and an already-reviewed TikTok integration; may offer scheduling and analytics. Costs include subscription fees, third-party account authorization/data handling, supported-account/media limitations and dependence on a vendor. Verify current TikTok support, developer approval, OAuth permissions and pricing before choosing. No vendor was audited or authorized in this work.

### Sources

- Intended use, unaudited restrictions and required UX: https://developers.tiktok.com/doc/content-sharing-guidelines/
- Direct Post prerequisites: https://developers.tiktok.com/doc/content-posting-api-get-started/
- Direct Post endpoint: https://developers.tiktok.com/doc/content-posting-api-reference-direct-post/
- Inbox upload and human completion: https://developers.tiktok.com/doc/content-posting-api-reference-upload-video/

## Readiness levels

Keep these distinct in any handoff:

1. Source and offline tests exist.
2. Dependencies installed and local CLI smoke-tested.
3. Human granted OAuth; expected channel verified read-only.
4. Separately approved private upload and readback succeeded.
5. Human reviewed video/rights/disclosures; separately approved publication succeeded and returned URL/status.

Only levels actually exercised may be reported. A mocked test or completed source file does not authorize or prove a live upload.
