#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import argparse
import os
import sys
import fnmatch

DEFAULT_EXCLUDED_DIRS = {
    ".git", "node_modules", "target", "bin", "obj", "build", "dist", "vendor", "__pycache__"
}

# Extensiones típicamente binarias o pesadas
DEFAULT_EXCLUDED_EXTS = {
    ".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp", ".ico", ".psd",
    ".pdf",
    ".zip", ".rar", ".7z", ".tar", ".gz", ".bz2", ".xz",
    ".mp3", ".wav", ".ogg", ".flac",
    ".mp4", ".mov", ".avi", ".mkv", ".webm",
    ".ttf", ".otf", ".woff", ".woff2",
    ".exe", ".dll", ".so", ".dylib", ".a", ".o",
    ".class", ".jar",
    ".blend", ".fbx", ".obj", ".glb", ".gltf"
}

def is_probably_text(path, sample_size=4096):
    """
    Heurística simple: si hay bytes nulos o demasiados bytes no imprimibles,
    asumimos que es binario.
    """
    try:
        with open(path, "rb") as f:
            chunk = f.read(sample_size)
    except Exception:
        return False

    if b"\x00" in chunk:
        return False

    # Porcentaje de bytes imprimibles (ASCII extendido + whitespace)
    text_chars = bytearray(range(32, 127)) + b"\n\r\t\f\b"
    if not chunk:
        return True
    printable = sum(c in text_chars for c in chunk)
    return (printable / len(chunk)) > 0.85

def match_any(patterns, name):
    return any(fnmatch.fnmatch(name, p) for p in patterns)

def should_skip_file(path, args):
    base = os.path.basename(path)

    # Ocultos
    if not args.show_hidden and base.startswith("."):
        return True

    # Tamaño
    try:
        if os.path.getsize(path) > args.max_bytes:
            return True
    except OSError:
        return True

    # Extensiones excluidas
    ext = os.path.splitext(base)[1].lower()
    if ext in DEFAULT_EXCLUDED_EXTS and not args.include:
        # Si el usuario especificó --include, respetamos su filtro y no saltamos por extensión
        return True

    # Patrones de exclusión
    if args.exclude and match_any(args.exclude, base):
        return True

    # Si hay --include, solo tomamos lo que matchee
    if args.include and not match_any(args.include, base):
        return True

    # Heurística de texto
    if not is_probably_text(path):
        return True

    return False

def walk_files(root, follow_symlinks=False):
    for dirpath, dirnames, filenames in os.walk(root, followlinks=follow_symlinks):
        # Filtrar directorios excluidos u ocultos
        pruned = []
        for d in list(dirnames):
            if d in DEFAULT_EXCLUDED_DIRS or d.startswith("."):
                pruned.append(d)
        for d in pruned:
            dirnames.remove(d)

        for fname in filenames:
            yield os.path.join(dirpath, fname)

def main():
    parser = argparse.ArgumentParser(
        description="Concatena contenidos de archivos de una carpeta en un .txt con formato 'nombre.ext:\\n<contenido>\\n\\n'."
    )
    parser.add_argument("folder", help="Carpeta raíz a procesar")
    parser.add_argument("-o", "--output", default="contenido_carpeta.txt",
                        help="Ruta del archivo de salida (.txt). Por defecto: contenido_carpeta.txt")
    parser.add_argument("--include", action="append", default=[],
                        help="Patrón(es) de archivos a incluir (glob). Ej: --include '*.rs' --include '*.py'")
    parser.add_argument("--exclude", action="append", default=[],
                        help="Patrón(es) de archivos a excluir (glob). Ej: --exclude '*.log'")
    parser.add_argument("--max-bytes", type=int, default=1_000_000,
                        help="Tamaño máximo por archivo (bytes). Por defecto: 1,000,000")
    parser.add_argument("--use-path", action="store_true",
                        help="Usar ruta relativa como encabezado en vez del nombre base.")
    parser.add_argument("--show-hidden", action="store_true",
                        help="Incluir archivos ocultos (nombres que empiezan con '.').")
    parser.add_argument("--follow-symlinks", action="store_true",
                        help="Seguir enlaces simbólicos al recorrer.")
    parser.add_argument("--encoding", default="utf-8",
                        help="Encoding para leer archivos de texto (con errors='replace'). Por defecto: utf-8")

    args = parser.parse_args()

    root = os.path.abspath(args.folder)
    if not os.path.isdir(root):
        print(f"✖ La ruta no es una carpeta: {root}", file=sys.stderr)
        sys.exit(1)

    count = 0
    with open(args.output, "w", encoding="utf-8", newline="\n") as out:
        for path in walk_files(root, follow_symlinks=args.follow_symlinks):
            if should_skip_file(path, args):
                continue

            rel = os.path.relpath(path, root)
            header = rel if args.use_path else os.path.basename(path)

            try:
                with open(path, "r", encoding=args.encoding, errors="replace") as f:
                    content = f.read()
            except Exception as e:
                # Si falla lectura como texto, lo saltamos
                continue

            # Escribir bloque
            out.write(f"{header}:\n")
            out.write(content)
            if not content.endswith("\n"):
                out.write("\n")
            out.write("\n")  # separador entre archivos

            count += 1

    print(f"✔ Listo. Archivos incluidos: {count}\n→ {args.output}")

if __name__ == "__main__":
    main()
