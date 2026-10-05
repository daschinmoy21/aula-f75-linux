# Upstream licensing follow-up

Checked on 2026-10-05 for [issue #4](https://github.com/daschinmoy21/aula-f75-linux/issues/4).

The upstream [Nokkasiili/aula-f75-linux](https://github.com/Nokkasiili/aula-f75-linux)
repository's `master` branch was at
[`33ae5bafb6de1bca45530dc28c95ac855be7dc51`](https://github.com/Nokkasiili/aula-f75-linux/tree/33ae5bafb6de1bca45530dc28c95ac855be7dc51).
Its tracked tree contains no licence or copying file, its README does not specify
licensing terms, and GitHub's repository metadata reports `license: null`
(the `/license` endpoint returns 404).

The licensing follow-up remains open. Before wider distribution, ask the upstream
maintainer to clarify the intended licence for the inherited driver, types, protocol
implementation, and sample configuration. No maintainer has been contacted as part
of this check, and no licence has been assigned to their code by this fork.
Record any response and the resulting licence/attribution requirements here.

The bundled `vendor/xattr` dependency retains its own MIT and Apache-2.0 licence
files. Those files do not establish a licence for the keyboard driver.
