---
"@googleworkspace/cli": patch
---

fix(auth): redirect OAuth2 URL prompt to stderr in proxy login flow. login_with_proxy_support() printed the authorization URL to stdout, which in non-TTY / scripted environments was buffered until the blocking TcpListener::accept() returned — leaving users with a hung process and no URL to open. Switching to eprintln! ensures the URL appears immediately, consistent with the CliFlowDelegate path.
