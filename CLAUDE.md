<!--
SPDX-FileCopyrightText: 2026 OpenCabinet contributors
SPDX-License-Identifier: CC-BY-SA-4.0
-->

# CLAUDE.md — Instrucciones para Claude Code

Este archivo le da contexto a Claude Code en cada sesión. El plan completo
(en español) está en [`docs/es/plan.md`](docs/es/plan.md).

## Qué es este proyecto

Suite open source, gratuita y modular para diseño, documentación y
fabricación de millwork (carpintería arquitectónica, cabinetry). Empieza por
un diseñador de gabinetes con despiece y lista de materiales; el CAD 2D propio
llega en fases posteriores.

El fundador (Andrés) es nuevo en open source. **Explica cada decisión en
español sencillo y con el porqué** antes de aplicarla.

## Fase actual: Fase 1a — Entorno del repositorio

La Fase 1 del plan se divide en dos partes:

- **1a — Entorno (actual):** repositorio, licencias, archivos de comunidad,
  CI de documentación y ADRs de las decisiones ya tomadas. **No se escribe
  código de la aplicación.**
- **1b — Diseño:** RFC-0001 (modelo de proyecto), ADR de unidades y
  precisión, ADR del stack con prototipos desechables comparables, RFC-0002
  (arquitectura de módulos y plugins). Recién aquí empieza el código.

La Fase 0 está cerrada: el contrato laboral fue revisado por un abogado y se
autorizó publicar.

Antes de cada paso relevante, confirma con Andrés. Trabaja en pasos pequeños
y haz un commit firmado (`git commit -s`) por paso.

## Principio de tamaño

"La estructura crece cuando crece la comunidad, no antes." No crear
documentos de proceso (MAINTAINERS, CODEOWNERS, CHANGELOG con entradas, repo
`.github` de la organización) hasta que exista algo que los necesite.

## Decisiones ya tomadas (no cambiarlas sin preguntar)

- Licencia del núcleo y módulos de escritorio: **GPL-3.0-or-later**.
- Servicios de servidor (futuros): **AGPL-3.0-or-later**.
- Documentación e íconos: **CC BY-SA 4.0**. Especificación del formato de
  archivo: **CC BY 4.0**. Plantillas de plugin: **MIT**.
- Archivos de configuración triviales (`.gitignore`, `.editorconfig`, etc.):
  **CC0-1.0**.
- Cumplimiento de licencias con la especificación **REUSE**
  (<https://reuse.software>), verificado en la CI con `reuse lint`. Solo se
  añade a `LICENSES/` una licencia cuando algún archivo la usa.
- Nombre del proyecto: **OpenCabinet**. Los módulos llevan nombres
  descriptivos con el prefijo de la suite: "OpenCabinet Designer",
  "OpenCabinet Shop Drawings" (ver ADR-0010).
- Nombre y logo: reservados por política de marca (TRADEMARKS.md).
- Contribuciones: **DCO** (`git commit -s`), sin CLA.
- Estructura: **monorepo**. Las plantillas de GitHub viven en el `.github/`
  del propio monorepo.
- Ramas: trunk-based; `main` protegida; todo entra por pull request; squash
  merge conservando las líneas `Signed-off-by`.
- Commits: **Conventional Commits**. Versionado: **SemVer**. CHANGELOG en
  formato Keep a Changelog (desde la primera versión).
- Idioma del repositorio: **inglés** para código, issues y documentación
  principal; documentos internos del fundador en `docs/es/`.
- ADRs en formato **MADR** dentro de `docs/adr/`.
- Unidades: **sistema imperial por defecto, métrico como opción** para el
  usuario. El modelo interno de precisión se decide en un ADR de la Fase 1b.
- Núcleo en **Rust** (ADR-0011): `cargo fmt`, `cargo clippy -D warnings` y
  pruebas en la CI. Las longitudes son enteros de 64 bits en nanómetros.

## Decisiones pendientes (no tomarlas por tu cuenta)

- GPL-3.0-or-later vs GPL-3.0-only (provisoriamente "or later").
- Plugins en el mismo proceso o aislados (RFC-0002).
- Tecnología de la interfaz gráfica de escritorio.
- Representación interna de longitudes y precisión (ADR-0009, propuesto:
  nanómetros enteros).

## Reglas de propiedad intelectual (obligatorias)

- Nunca copiar código, íconos, textos, plantillas, bibliotecas ni
  documentación de AutoCAD, Bluebeam, Microvellum, Cabinet Vision, Mozaik u
  otro software propietario.
- No usar marcas de terceros en nombres, logos o identificadores del
  proyecto.
- Los archivos de prueba se crean desde cero para el proyecto.
- Cada archivo lleva encabezado SPDX (`SPDX-FileCopyrightText` y
  `SPDX-License-Identifier`); los que no admiten comentarios se declaran en
  `REUSE.toml`.
- Textos de licencias (GPL, CC, MIT, Contributor Covenant) deben ser los
  oficiales completos, sin modificar, descargados de la fuente oficial.

## Cuentas de GitHub

- El proyecto usa **solo la cuenta personal `A-newGP`**. Nunca usar la
  cuenta laboral `agonzales-metric` ni la organización de su empleador para
  nada de este proyecto (ni push, ni issues, ni forks).
- Antes de cualquier operación con `gh`, verificar con `gh auth status` qué
  cuenta está activa.

## Lo que Claude Code NO debe hacer

- Crear la organización de GitHub o cambiar ajustes de seguridad del repo.
  Esos pasos los hace Andrés en la web; tú le das la guía paso a paso
  ([`docs/es/guia-github.md`](docs/es/guia-github.md)).
- Hacer push sin permiso explícito de Andrés en esa sesión.
- Manejar contraseñas, tokens ni secretos.
- Escribir código de la aplicación durante la Fase 1a.
