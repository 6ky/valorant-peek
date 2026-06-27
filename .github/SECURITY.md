# Security Policy

Peek runs entirely on your machine. It reads your local Riot client and talks to
Riot's public endpoints, and it sends nothing anywhere else. The security
surface that matters is the handling of the local API token and the static data
it caches to disk.

## Supported versions

Only the latest release gets fixes. Please update before reporting.

## Reporting a vulnerability

Do not open a public issue for a security problem. Report it privately through
GitHub: open the Security tab and choose "Report a vulnerability". Include what
you found, how to reproduce it, and the impact you think it has.

You will get a response as soon as I can manage. Once a fix ships, the report
can be made public so other people understand what changed.

## Out of scope

Peek does not modify the game, read its memory, or run an overlay. Anything
about injecting into or automating the game is not something this project does
or will take on.
