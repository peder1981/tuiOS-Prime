# Spec — tuios-apps (Nível C, Fase 2: c-tui)

> **Data:** 2026-10-03 · **Status:** design conduzido sob autorização do operador
> ("prossiga com todas as demais fases") · **Escopo:** sub-projeto `c-tui`
> **Relacionado:** spec Fase 1 (c-core) — `2026-10-03-tuios-apps-fase1-c-core-design.md`

## 1. Contexto e visão

A Fase 1 (`c-core`) entregou o núcleo CLI testável (R1–R13): manifesto,
discovery, runner, pacote e CLI com exit codes. A Fase 2 (`c-tui`) fecha o
Nível C para o **usuário final do tuiOS**: menu dialog (F5), app-exemplo de
fábrica na ISO (F6), cópia de apps pelo instalador ao destino, guia de
documentação (F8) e evidência no smoke T2.

**Decisões herdadas (spec Fase 1):** menu é camada fina consumindo os mesmos
módulos; exemplos em `apps/exemplos/` copiados para `/opt/tuios/apps` via
Nix; stack Python 3 stdlib + `dialog` (zero deps PyPI).

**Semântica tmpfiles `C` (man tmpfiles.d do host, verificada):** copia
recursivamente só se o destino não existe ou está vazio; se não-vazio, a
cópia inteira é **pulada** — conteúdo do usuário nunca é sobrescrito, e o
exemplo de fábrica volta no boot seguinte se apagado por completo.

## 2. Escopo

**Dentro (Fase 2 `c-tui`):**
- F5 `tuios-apps menu` — menu dialog camada fina sobre os módulos
- F6 `apps/exemplos/ola-tuios/` versionado → `/opt/tuios/apps` na live (tmpfiles)
- Instalador copia os roots de sistema ao destino (`instalar.sh`)
- T2 do `install.assert.sh` prova o exemplo listado
- F8 guia `docs/tuios-apps.md` + ponte no README
- Menu exercitado no `apps.assert.sh` com dialog fake (sem terminal)

**Fora (Fase 3):** origem git/URL em `adicionar`, índice remoto (store HTTP).
**Já entregue (Fase 1):** R1–R13.

## 3. Requisitos funcionais

| ID | Requisito | Critério de aceite |
|----|-----------|--------------------|
| R14 | Menu dialog | `tuios-apps menu` invoca `dialog` via `subprocess` (sem shell), com `--stdout`; `TUIOS_DIALOG` sobrepõe o binário (testes/CI); `dialog` ausente → exit 4 com mensagem PT-BR; `Sair` → exit 0; **nunca** stacktrace — erros de domínio viram `--msgbox` e o menu continua |
| R15 | Paridade com a CLI | Menu reusa `descobrir/buscar/rodar/adicionar` e as mesmas regras (sombra usuário>sistema, exits); **listar** mostra sistema+usuário; **instalar/remover** pelo menu operam apenas o root do **usuário** (operações `--sistema` ficam só na CLI — remover de sistema pelo menu mostra dica com o comando); **rodar** funciona para qualquer origem |
| R16 | Exemplo de fábrica na ISO | `apps/exemplos/ola-tuios/` versionado no repo; derivação `nixos/apps/exemplos.nix` (`runCommand`); live: tmpfiles `d /opt/tuios/apps` + `C /opt/tuios/apps/ola-tuios - - - - <storePath>` — copia no primeiro boot, pula se destino não-vazio, restaura se apagado |
| R17 | Instalador copia ao destino | `gerar_config()` em `installer/lib/instalar.sh` executa `cp -a /opt/tuios/apps/. /mnt/opt/tuios/apps/` quando o root existir (idempotente, antes do `nixos-install`) |
| R18 | T2 prova o exemplo | assert do T2 troca o `[]` por: JSON contém `"nome": "ola-tuios"` **e** `"origem": "sistema"` (exemplo instalado veio da live) |
| R19 | Documentação | `docs/tuios-apps.md`: conceito, manifesto, CLI, menu, ciclo de pacote, roots/sombra, exemplos, exit codes, como testar; README ganha link/ponte |
| R20 | Menu testável sem terminal | pytest: `TUIOS_DIALOG` = script fake com **fila** de respostas (`rc|stdout` por chamada) + log de argv; `apps.assert.sh` ganha smoke do menu com o mesmo fake; `just test-all` e `just test-install` verdes |

## 4. Interface (adições à Fase 1)

```
tuios-apps menu
```

**Exit codes do menu:** `0` sair/ok · `1` erro geral · `4` `dialog` ausente.
(As ações do menu capturam erros de domínio em `--msgbox`; os exit codes
2/3/4 da CLI permanecem exclusivos da interface CLI.)

**Menu (tags estáveis, labels PT-BR):**

```
principal:  listar | instalar | sair
listar:     <nome-app>... | voltar      (itens: "<versao> — <descricao> [<origem>]")
app:        detalhes | rodar | remover | voltar
instalar:   --fselect (caminho .tar.gz) → adicionar() → --msgbox resultado
remover:    --yesno confirmação → rmtree (apenas origem "usuario")
```

Ciclo: principal → listar → app → voltar → principal → sair.

## 5. Modelo de dados

Sem mudança (manifesto `tuios-app.toml`, `App`, pacote `tar.gz`+`SHA256SUMS`).
Exemplo de fábrica = o mesmo app do assert da Fase 1:

```toml
nome = "ola-tuios"
versao = "1.0.0"
descricao = "Primeiro app tuiOS"
entry = "ola.prw"
categoria = "exemplo"
```

**Fila do dialog fake (contrato de teste):** cada linha de
`FAKE_QUEUE` = `<rc>|<stdout>`; a cada invocação consome 1 linha; fila vazia
→ rc 1 (cancel). `FAKE_LOG` acumula `argv[1:]` por chamada (1 linha).

## 6. Arquitetura

```
apps/tuios-apps/tuiosapps/menu.py   # NOVO: DialogAusente, _resolver_dialog,
                                     #       _dialog/_msgbox, _tela_* , rodar_menu()
apps/tuios-apps/tuiosapps/cli.py     # MOD: subcomando `menu` + except DialogAusente → 4
apps/exemplos/ola-tuios/             # NOVO: tuios-app.toml + ola.prw (fonte versionada)
nixos/apps/exemplos.nix              # NOVO: runCommand "tuios-exemplos"
flake.nix                            # MOD: tuiosExemplos (let + packages + specialArgs)
nixos/iso.nix                        # MOD: arg tuiosExemplos + tmpfiles d/C
installer/lib/instalar.sh            # MOD: cp -a dos roots ao destino
scripts-assert/install.assert.sh     # MOD: T2 assert → ola-tuios/sistema
scripts-assert/apps.assert.sh        # MOD: smoke do menu (dialog fake)
docs/tuios-apps.md                   # NOVO: guia (R19)
README.md                            # MOD: ponte para o guia
```

- `menu.py` **não** importa `cli.py` (evita ciclo): destino de instalação
  vem de `discovery.roots()`; raiz do usuário via filtro `origem == "usuario"`.
- `dialog` é dependência de runtime do sistema (já em
  `environment.systemPackages` da ISO e do destino) — o pacote Python não o
  embute; ausência → exit 4 (mesmo padrão do `advplc`).

## 7. Segurança

- `subprocess` sempre com lista de args (`shell=False`); sem interpolação.
- Menu não expõe operação `--sistema` (reduz superfície; CLI continua sendo
  o caminho administrativo, protegida por `os.geteuid()`).
- `--fselect` entrega caminho arbitrário ao usuário — validação é a mesma do
  CLI (`adicionar`: staging, prefixo único, traversal, checksum, manifesto).

## 8. Testes e critérios de aceite (F7)

| Camada | Conteúdo |
|--------|----------|
| pytest `tests/test_menu.py` | dialog ausente → 4; sair → 0; listar exibe apps do root de teste; detalhes/msgbox; rodar propaga exit code; remover com yesno (usuário) e bloqueio de sistema com dica; instalar via fselect |
| `apps.assert.sh` | smoke: fila `listar → voltar → sair` com `TUIOS_DIALOG` fake; exit 0 + `--menu` no log + `ola-tuios` listado |
| QEMU T2 | `listar --json` contém `ola-tuios` com `origem: sistema` no sistema instalado |
| Regressão | `just test-all` e `just test-install` verdes; ISO nova com exemplo |

**Aceite da Fase 2:** todos os R14–R20 verificados por teste automatizado;
`just test-all` e `just test-install` verdes; exemplo presente na live e no
destino; guia publicado.

## 9. Fronteiras com outras fases

- **Fase 3:** `adicionar` ganha origem git/URL e índice remoto; o menu
  "Instalar" continua sendo arquivo local (extensão de URL fica para a
  Fase 3, se desejada).
- **Fase 1:** nenhuma mudança de comportamento — R1–R13 intactos (regressão
  obrigatória).
