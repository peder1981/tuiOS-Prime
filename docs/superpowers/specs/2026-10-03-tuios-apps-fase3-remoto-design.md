# tuios-apps — Fase 3 (remoto): origens URL/git + índice remoto

> Spec de design · 03/10/2026 · complementa a
> [Fase 1 (c-core)](2026-10-03-tuios-apps-fase1-c-core-design.md) (R1–R13) e a
> [Fase 2 (c-tui)](2026-10-03-tuios-apps-fase2-c-tui-design.md) (R14–R20).

## Objetivo

Levar apps de **origem remota** ao mesmo ciclo já entregue: baixar de
`http(s)://`/`file://`, clonar repositório git, e instalar a partir de um
**índice remoto** com verificação de `sha256` — sempre sob comando explícito
do usuário, com timeout, sem mudança no schema do manifesto.

## Requisitos

### R21 — `adicionar` com origem URL

- `tuios-apps adicionar <url>` aceita `http://`, `https://` e `file://`
  apontando para um `.tar.gz`.
- Download para arquivo temporário via `urllib.request` com **timeout de 30 s**
  (`urlopen(..., timeout=30)`), `shell=False`, sem interpolação.
- Download ok → encaminha para o `adicionar` de pacote existente (validação
  completa de staging da Fase 1, inalterada).
- Falha de rede/timeout/HTTP ≠ 2xx → mensagem clara, **exit 1**.
- Pacote baixado inválido → **exit 2** (semântica já definida).
- Caminho local de arquivo continua aceito (comportamento da Fase 1 intacto).

### R22 — `adicionar` com origem git

- `tuios-apps adicionar <url-git>` aceita origens que terminam em `.git`
  (local, `file://...git` ou `http(s)://...git`).
- `git clone --depth 1` para diretório temporário, com
  `GIT_TERMINAL_PROMPT=0` (nunca pendurar pedindo credencial) e timeout.
- Clone ok → `empacotar` o working tree (regras de exclusão da Fase 1) →
  `adicionar` o tarball gerado → limpeza garantida (`finally`).
- `git` ausente no PATH → **exit 4** (dependência).
- Falha de clone (rede/repo inexistente) → **exit 1**.
- `.git` do repositório **não** entra no pacote (exclusão padrão da Fase 1).

### R23 — `atualizar-indice`

- `tuios-apps atualizar-indice <url>` baixa um `indice.toml` (timeout 30 s)
  e o grava em `${XDG_DATA_HOME:-~/.local/share}/tuios/indice.toml`.
- Escrita **atômica** (arquivo temporário no mesmo diretório + `os.replace`).
- Schema do índice (novo; manifesto de app **não** muda — R21-R26 não tocam
  em R1-R13):

  ```toml
  [[app]]
  nome = "ola-tuios"        # mesmo slug do manifesto
  versao = "1.0.0"
  descricao = "Primeiro app"
  url = "https://exemplo/ola-tuios-1.0.0.tar.gz"   # http(s) ou file://
  sha256 = "64 hex"         # sha256 do tarball
  ```

- Entrada sem campos obrigatórios, `sha256` fora do padrão de 64 hex ou
  URL sem esquema suportado → **exit 2** (índice inválido, nada é gravado).
- Falha de download → **exit 1**.

### R24 — `buscar`

- `tuios-apps buscar <termo> [--json]` filtra o índice local por `nome`
  e `descricao` (case-insensitive, substring).
- Sem índice → mensagem orientando `atualizar-indice`, **exit 1**.
- Saída: `NOME VERSAO DESCRICAO` (alinhada, padrão do `listar`).

### R25 — `instalar <nome>`

- `tuios-apps instalar <nome>`: procura no índice local (não achado →
  **exit 3**), baixa o tarball da `url` (timeout 30 s), confere **sha256
  do arquivo baixado** contra o do índice (divergência → **exit 2**, baixado
  é descartado), e então delega ao `adicionar` (instala no root de usuário,
  mesma sombra da Fase 1; `--sistema` idem, exige root).
- Já instalado: o `adicionar` da Fase 1 decide (sobreposição por sombra) —
  sem regra nova.

### R26 — Rede só sob comando explícito + timeouts

- Rede **nunca** acontece em `listar`/`info`/`rodar`/`validar`/`empacotar`/
  `remover`/`menu` — apenas em `adicionar <url>`, `atualizar-indice` e
  `instalar`.
- Toda chamada de rede tem timeout explícito (30 s); `git clone` com timeout
  (120 s) via `subprocess.run(..., timeout=120)`.
- Nenhum shell: URLs vão como argumento único de lista.

## Não-escopo

- Menu (Fase 2) fica como está — sem telas novas nesta fase.
- Sem assinatura/PGP, sem mirror, sem retry automático, sem cache de download.
- Schema do manifesto `tuios-app.toml` não muda.

## Contrato de código

- Módulo novo `origem.py`: `OrigemInvalida/RedeFalhou/GitAusente` +
  `materializar(url, destino_temp) -> Path` (tarball) — download ou clone+empacotar.
- Módulo novo `indice.py`: `ErroIndice/IndiceNaoEncontrado` +
  `atualizar(url)/carregar()/buscar(termo)/lookup(nome) -> Entrada` +
  `baixar_e_conferir(entrada, tmp) -> Path` (sha256).
- CLI: `adicionar` estendido (URL/git), `atualizar-indice`, `buscar`,
  `instalar`; exit codes reaproveitados 0/1/2/3/4.
- Exceções novas mapeadas no `main()`: `RedeFalhou/OrigemInvalida` → 1,
  `ErroIndice` → 2, `IndiceNaoEncontrado`/`AppNaoEncontrado` → 3,
  `GitAusente` → 4.
- Testes: download ok via `file://`; falhas por monkeypatch de `urlopen`;
  caminho http local com `http.server` em thread (skipped se socket indisponível);
  git real com `skipif shutil.which("git")`; índices inválidos e sha256 divergente.
- `apps.assert.sh` ganha seção: empacotar → `adicionar file://...tar.gz` →
  app instalado (E2E sem rede).

## Escala de confiança

- 🟢 R21-R26 derivam das decisões do brainstorm Nível C (decomposição
  c-core → c-tui → remoto) já registradas em mem0.
- 🟡 Esquema do `indice.toml` é decisão de design desta spec — validável
  na revisão.
- 🟡 Loopback no sandbox Nix: teste de http local pode ser marcado skip
  se o sandbox bloquear; caminho `file://` é a prova obrigatória.
