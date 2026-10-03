"""CLI tuios-apps — argparse + exit codes 0/1/2/3/4 + saída --json (R6, R12, R13)."""
from __future__ import annotations

import argparse
import json
import os
import shutil
import sys
from pathlib import Path

from . import __version__
from .discovery import App, buscar, descobrir
from .manifest import ErroManifesto, carregar_manifesto
from .package import ErroPacote, adicionar, empacotar
from .runner import AdvplcAusente, rodar

EXIT_ERRO = 1
EXIT_INVALIDO = 2  # manifesto/pacote inválido
EXIT_NAO_ENCONTRADO = 3
EXIT_DEPENDENCIA = 4


class AppNaoEncontrado(Exception):
    """App inexistente — exit 3."""


def _erro(msg: str) -> None:
    print(f"tuios-apps: erro: {msg}", file=sys.stderr)


def _dirs_destino(sistema: bool) -> Path:
    from .discovery import roots

    alvo = "sistema" if sistema else "usuario"
    for origem, caminho in roots():
        if origem == alvo:
            return caminho
    raise AssertionError("roots() sem a origem pedida")


def _cmd_listar(args: argparse.Namespace) -> int:
    apps = descobrir()
    if args.json:
        print(
            json.dumps(
                [
                    {
                        "nome": a.nome,
                        "versao": a.versao,
                        "descricao": a.descricao,
                        "categoria": a.categoria,
                        "origem": a.origem,
                        "path": str(a.path),
                    }
                    for a in apps
                ],
                ensure_ascii=False,
                indent=2,
            )
        )
        return 0
    if apps:
        print(f"{'NOME':<16} {'VERSAO':<8} {'ORIGEM':<8} DESCRICAO")
        for a in apps:
            print(f"{a.nome:<16} {a.versao:<8} {a.origem:<8} {a.descricao}")
    return 0


def _cmd_info(args: argparse.Namespace) -> int:
    app = buscar(args.nome)
    if app is None:
        raise AppNaoEncontrado(f"app nao encontrado: {args.nome}")
    print(f"nome: {app.nome}")
    print(f"versao: {app.versao}")
    print(f"descricao: {app.descricao}")
    print(f"categoria: {app.categoria or '-'}")
    print(f"autor: {app.autor or '-'}")
    print(f"origem: {app.origem}")
    print(f"entry: {app.entry}")
    print(f"path: {app.path}")
    return 0


def _cmd_rodar(args: argparse.Namespace) -> int:
    app = buscar(args.nome)
    if app is None:
        raise AppNaoEncontrado(f"app nao encontrado: {args.nome}")
    restante = list(args.resto)
    if restante and restante[0] == "--":
        restante = restante[1:]
    return rodar(app, restante)


def _cmd_validar(args: argparse.Namespace) -> int:
    m = carregar_manifesto(Path(args.dir))
    print(f"OK: {m.nome} {m.versao} (entry: {m.entry})")
    return 0


def _cmd_empacotar(args: argparse.Namespace) -> int:
    saida = empacotar(Path(args.dir), Path(args.saida) if args.saida else None)
    print(saida)
    return 0


def _cmd_adicionar(args: argparse.Namespace) -> int:
    if args.sistema and os.geteuid() != 0:
        raise PermissionError("adicionar --sistema exige root (use sudo)")
    destino = _dirs_destino(args.sistema)
    alvo = adicionar(Path(args.tarball), destino)
    print(alvo)
    return 0


def _cmd_remover(args: argparse.Namespace) -> int:
    if args.sistema and os.geteuid() != 0:
        raise PermissionError("remover --sistema exige root (use sudo)")
    origem_desejada = "sistema" if args.sistema else "usuario"
    alvo = next(
        (a for a in descobrir() if a.nome == args.nome and a.origem == origem_desejada),
        None,
    )
    if alvo is None:
        raise AppNaoEncontrado(
            f"app nao encontrado: {args.nome} (origem {origem_desejada})"
        )
    shutil.rmtree(alvo.path)
    print(alvo.path)
    return 0


def _montar_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="tuios-apps",
        description="Apps AdvPL de primeira classe no tuiOS (Nível C, fase 1)",
    )
    parser.add_argument("--version", action="version", version=f"tuios-apps {__version__}")
    sub = parser.add_subparsers(dest="comando", required=True)

    p = sub.add_parser("listar", help="lista apps instalados")
    p.add_argument("--json", action="store_true", help="saída JSON")
    p.set_defaults(func=_cmd_listar)

    p = sub.add_parser("info", help="mostra detalhes de um app")
    p.add_argument("nome")
    p.set_defaults(func=_cmd_info)

    p = sub.add_parser("rodar", help="executa um app via advplc")
    p.add_argument("nome")
    p.add_argument("resto", nargs=argparse.REMAINDER, help="argumentos após --")
    p.set_defaults(func=_cmd_rodar)

    p = sub.add_parser("validar", help="valida o manifesto de um diretório")
    p.add_argument("dir", nargs="?", default=".")
    p.set_defaults(func=_cmd_validar)

    p = sub.add_parser("empacotar", help="gera app.tar.gz a partir do diretório")
    p.add_argument("dir")
    p.add_argument("-o", "--saida", default=None, help="arquivo de saída")
    p.set_defaults(func=_cmd_empacotar)

    p = sub.add_parser("adicionar", help="instala um pacote .tar.gz")
    p.add_argument("tarball")
    p.add_argument("--sistema", action="store_true", help="instala em /opt/tuios/apps (root)")
    p.set_defaults(func=_cmd_adicionar)

    p = sub.add_parser("remover", help="remove um app instalado")
    p.add_argument("nome")
    p.add_argument("--sistema", action="store_true", help="remove de /opt/tuios/apps (root)")
    p.set_defaults(func=_cmd_remover)

    return parser


def main(argv: list[str] | None = None) -> int:
    """Entry point: devolve o exit code (nunca lança para erro de uso)."""
    args = _montar_parser().parse_args(argv)
    try:
        return args.func(args)
    except (ErroManifesto, ErroPacote) as exc:
        _erro(str(exc))
        return EXIT_INVALIDO
    except AppNaoEncontrado as exc:
        _erro(str(exc))
        return EXIT_NAO_ENCONTRADO
    except AdvplcAusente as exc:
        _erro(str(exc))
        return EXIT_DEPENDENCIA
    except PermissionError as exc:
        _erro(str(exc))
        return EXIT_ERRO
    except BrokenPipeError:
        return 0
    except OSError as exc:
        _erro(f"falha de sistema: {exc}")
        return EXIT_ERRO
