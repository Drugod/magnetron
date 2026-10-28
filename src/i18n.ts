export type Language = "es" | "en";
export type Appearance = "system" | "light" | "dark";
function saved(key: string) {
  try {
    return typeof localStorage === "undefined"
      ? null
      : localStorage.getItem(key);
  } catch {
    return null;
  }
}
function persist(key: string, value: string) {
  try {
    if (typeof localStorage !== "undefined") localStorage.setItem(key, value);
  } catch {
    /* Ephemeral mode still supports switching. */
  }
}
let language: Language = saved("magnetron.language") === "en" ? "en" : "es";
let appearance: Appearance = ["light", "dark"].includes(
  saved("magnetron.appearance") ?? "",
)
  ? (saved("magnetron.appearance") as Appearance)
  : "system";
export const getLanguage = () => language;
export const locale = () => (language === "en" ? "en-GB" : "es-ES");
const pairs: Record<string, string> = {
  "Calidad de YouTube": "YouTube quality",
  "Mejor disponible": "Best available",
  "Se descargará hasta la calidad elegida, según lo disponible en el vídeo.": "Downloads up to the selected quality, depending on what the video offers.",
  "Pega enlaces eD2k, magnet o YouTube, uno por línea": "Paste eD2k, magnet or YouTube links, one per line",
  "YouTube: se ha recuperado la última copia válida de la cola. Los archivos descargados se conservan.": "YouTube: the last valid queue checkpoint was recovered. Downloaded files are preserved.",
  "YouTube: no se pudo recuperar la cola. Se ha conservado una copia de los datos dañados y no se han borrado archivos descargados.": "YouTube: the queue could not be recovered. A copy of the damaged data was preserved and downloaded files were not deleted.",
  Entendido: "Understood",
  "BitTorrent: se ha recuperado la última copia válida de la cola. Los archivos descargados se conservan.":
    "BitTorrent: the last valid queue checkpoint was recovered. Downloaded files are preserved.",
  "BitTorrent: no se pudo recuperar la cola. Se ha conservado una copia de los datos dañados y no se han borrado archivos descargados.":
    "BitTorrent: the queue could not be recovered. A copy of the damaged data was preserved and downloaded files were not deleted.",
  "BitTorrent v2: se ha recuperado la última copia válida de la cola. Los archivos descargados se conservan.":
    "BitTorrent v2: the last valid queue checkpoint was recovered. Downloaded files are preserved.",
  "BitTorrent v2: no se pudo recuperar la cola. Se ha conservado una copia de los datos dañados y no se han borrado archivos descargados.":
    "BitTorrent v2: the queue could not be recovered. A copy of the damaged data was preserved and downloaded files were not deleted.",
  "eD2k: se ha recuperado la última copia válida de la cola. Los archivos descargados se conservan.":
    "eD2k: the last valid queue checkpoint was recovered. Downloaded files are preserved.",
  "eD2k: no se pudo recuperar la cola. Se ha conservado una copia de los datos dañados y no se han borrado archivos descargados.":
    "eD2k: the queue could not be recovered. A copy of the damaged data was preserved and downloaded files were not deleted.",

  "Preferencias rápidas": "Quick preferences",
  "Las actualizaciones se comprueban automáticamente en Windows.":
    "Updates are checked automatically on Windows.",
  "En este sistema, descarga las nuevas versiones desde GitHub.":
    "On this system, download new versions from GitHub.",
  "Actualización disponible": "Update available",
  "Actualizaciones de MAGNETRON": "MAGNETRON updates",
  "El canal público se activará al publicar la primera versión.":
    "The public update channel will be enabled with the first published release.",
  "Comprobar actualizaciones": "Check for updates",
  "Instalar actualización y cerrar": "Install update and close",
  "Comprobando…": "Checking\u2026",
  "Actualización verificada y preparada": "Update verified and ready",
  "Ya tienes la última versión": "You have the latest version",
  "BitTorrent v2: metadatos verificados y una fuente completa conectada. Su disponibilidad puede cambiar.":
    "BitTorrent v2: verified metadata and a connected complete source. Availability may change.",
  Favoritos: "Favourites",
  "Guardar favorito": "Save favourite",
  "Quitar de favoritos": "Remove favourite",
  "Todavía no hay favoritos": "No favourites yet",
  "Guarda resultados con la estrella para encontrarlos aquí.":
    "Star results to find them here.",
  "Guardado · vuelve a comprobar al descargar":
    "Saved · availability checked again when downloading",
  "Formato de exportación": "Export format",
  Enlaces: "Links",
  Exportar: "Export",
  "Exportar resultados": "Export results",
  "Resultados exportados": "Results exported",
  "Carpetas compartidas": "Shared folders",
  "Solo se comparten las carpetas que actives, incluidos sus subdirectorios. BitTorrent crea un enlace magnet para compartir esa carpeta; eD2k publica sus archivos. Durante las descargas se siguen intercambiando piezas.":
    "Only folders you enable are shared, including subfolders. BitTorrent creates a magnet link for the folder; eD2k publishes its files. Pieces are still exchanged during downloads.",
  "Añadir carpeta": "Add folder",
  "Actualizar archivos compartidos": "Refresh shared files",
  "Compartición actualizada": "Sharing refreshed",
  "Copiar enlace magnet": "Copy magnet link",
  "Preparando torrent": "Preparing torrent",
  "Compartiendo en eD2k": "Sharing on eD2k",
  "Sin compartir": "Not shared",
  "Compartiendo en ambas redes": "Sharing on both networks",
  "Compartiendo en BitTorrent": "Sharing on BitTorrent",
  "Error en BitTorrent": "BitTorrent error",
  "Índice y recursos": "Index and resources",
  "Rastreo DHT programado": "Scheduled DHT discovery",
  "Desde la hora (0–23)": "From hour (0–23)",
  "Hasta la hora (0–24)": "Until hour (0–24)",
  "Máximo de registros": "Maximum records",
  "Presupuesto de metadatos (MiB)": "Metadata budget (MiB)",
  "Pausa entre rastreos (segundos)": "Pause between lookups (seconds)",
  "Tiempo por recuperación (segundos)": "Recovery timeout (seconds)",
  "Una recuperación simultánea limita CPU y red. El presupuesto corresponde al texto de metadatos; SQLite, su índice y los temporales ocupan espacio adicional.":
    "One recovery at a time bounds CPU and network activity. The budget covers metadata text; SQLite, its index and temporary files take additional space.",
  Explorar: "Explore",
  Descargas: "Downloads",
  Buscar: "Search",
  Ajustes: "Settings",
  Fuentes: "Sources",
  Automáticas: "Automatic",
  "Fuentes integradas": "Sources",
  "Fuentes automáticas": "Automatic sources",
  "Buscar o pegar enlaces…": "Search or paste links…",
  "Buscar o pegar enlaces": "Search or paste links",
  "Buscar o descargar": "Search or download",
  "Limpiar búsqueda": "Clear search",
  "MAGNETRON inicio": "MAGNETRON home",
  Idioma: "Language",
  Apariencia: "Appearance",
  Claro: "Light",
  Oscuro: "Dark",
  Sistema: "System",
  Personalización: "Personalisation",
  "DHT pausada. Se activa manualmente y no descarga contenido.":
    "DHT paused. Enable it manually; it only retrieves metadata.",
  "Conectando con la red DHT…": "Connecting to the DHT network…",
  "DHT pausada. El índice se conserva; al buscar se comprobará su disponibilidad.":
    "DHT paused. The index is kept; availability is checked during search.",
  "Sin nodos accesibles; reintentando…": "No reachable nodes; retrying…",
  "Descubriendo identificadores y recuperando solo metadatos.":
    "Discovering identifiers and retrieving metadata only.",
  "No se pudo conectar con DHT. Comprueba que tu red permite UDP y vuelve a activar el índice.":
    "Could not connect to DHT. Check that your network allows UDP, then enable discovery again.",
  "Al cerrar la ventana, las descargas continúan en la bandeja del sistema. Para cerrar el programa, elige Salir en su icono.":
    "Closing the window keeps downloads running in the system tray. To exit the app, choose Quit from its tray icon.",
  "Filtrar por tipo de contenido": "Filter by content type",
  "Ordenar resultados": "Sort results",
  "Vista previa de interfaz. La búsqueda real y SQLite funcionan en la aplicación de escritorio. Ejecuta":
    "Interface preview. Live search and SQLite run in the desktop app. Run",
  "Buscar torrents": "Search torrents",
  "El destino no es una carpeta": "The destination is not a folder",
  "Ya existe un archivo con ese nombre; no se sobrescribe. Cambia su nombre o elige otra carpeta":
    "A file with this name already exists. Rename it or choose another folder; it will not be overwritten.",
  "La fuente requiere configuración, no responde o bloquea el acceso. No se eluden sus controles.":
    "The source needs configuration, is unavailable or blocks access. Its access controls are respected.",
  "Búsqueda completada": "Search complete",
  "Búsqueda finalizada": "Search finished",
  "Resultados parciales": "Partial results",
  "Descubrimiento pausado": "Discovery paused",
  "DHT pausada": "DHT paused",
  "Sin activar": "Not enabled",
  "Iniciando DHT…": "Starting DHT…",
  "Comprobando disponibilidad…": "Checking availability…",
  "Tiempo de comprobación agotado · resultados parciales":
    "Availability check timed out · partial results",
  "Búsqueda cancelada": "Search cancelled",
  "Error de búsqueda": "Search error",
  Aviso: "Notice",
  "Comprobación terminada": "Check complete",
  "Listo para explorar": "Ready to explore",
  "Catálogos públicos, resultados reales.": "Public catalogues. Real results.",
  "En espera": "Ready",
  "Resultados de búsqueda": "Search results",
  "Todos los ficheros": "All files",
  "ISO / imágenes de disco": "ISO / disk images",
  Vídeo: "Video",
  Documentos: "Documents",
  Imágenes: "Images",
  Comprimidos: "Archives",
  "Programas / juegos": "Software / games",
  "Otros ficheros": "Other files",
  "Sin clasificar": "Uncategorised",
  "Orden: relevancia": "Sort: relevance",
  "Orden: tamaño": "Sort: size",
  "Orden: recientes": "Sort: newest",
  "Orden: nombre": "Sort: name",
  Audio: "Audio",
  "El tipo se obtiene de la categoría de la fuente o de las extensiones conocidas. Los paquetes mixtos pueden aparecer en varios tipos.":
    "Types come from source categories or known extensions. Mixed collections may appear under several types.",
  "Empieza por una buena búsqueda.": "Find something worth downloading.",
  "Consultamos las fuentes por ti y agrupamos las coincidencias.":
    "We search the sources and bring the matches together.",
  "Los resultados aparecerán aquí, conforme lleguen.":
    "Results will appear here as they arrive.",
  "Escribe lo que quieres encontrar y pulsa Buscar. Después puedes filtrar los resultados.":
    "Search for a file or paste an eD2k or magnet link. One field, everything you need.",
  "Buscar archivos o pegar enlaces. Así de simple.":
    "Search for files or paste links. That simple.",
  "Fuentes de búsqueda": "Search sources",
  "Recursos y privacidad": "Resources and privacy",
  "Tus consultas se envían a los operadores de las fuentes activadas.":
    "Queries are sent to the operators of enabled sources.",
  "Imágenes oficiales · releases.ubuntu.com":
    "Official images · releases.ubuntu.com",
  "Instalación y Live · cdimage.debian.org":
    "Installation and Live · cdimage.debian.org",
  "Catálogo público · libros, audio, vídeo y software":
    "Public catalogue · books, audio, video and software",
  "Conservar índice local": "Keep local index",
  "Busca también metadatos guardados sin aplicarles caducidad.":
    "Also search saved metadata without expiring it.",
  "Guardar historial": "Save search history",
  "Solo en este equipo, hasta 100 búsquedas.":
    "On this device only, up to 100 searches.",
  "Fuentes simultáneas": "Simultaneous sources",
  "Tiempo por fuente (segundos)": "Source timeout (seconds)",
  "Caducidad de caché (horas)": "Cache lifetime (hours)",
  "Guardar ajustes": "Save settings",
  "Borrar caché e historial": "Clear cache and history",
  "Cerrar ajustes": "Close settings",
  "Volver a buscar": "Back to search",
  "Cerrar fuentes": "Close sources",
  "Carpeta de descarga": "Download folder",
  "Se preguntará en la primera descarga":
    "Choose when you download your first file",
  "Elegir carpeta": "Choose folder",
  Quitar: "Clear",
  "Los archivos se guardan directamente aquí, sin carpetas añadidas por MAGNETRON.":
    "Files are saved directly here, without extra MAGNETRON folders.",
  "Límite de bajada (KiB/s)": "Download limit (KiB/s)",
  "Límite de subida (KiB/s)": "Upload limit (KiB/s)",
  "0 = sin límite. Mínimo limitado: 32 KiB/s. Se reparte entre BitTorrent y eD2k cuando ambas redes están activas.":
    "0 = unlimited. Minimum limit: 32 KiB/s. Shared between BitTorrent and eD2k when both networks are active.",
  "El descubrimiento DHT se activa en «Fuentes». Funciona solo mientras MAGNETRON está abierto y se pausa al cerrar. BitTorrent y eD2k descargan dentro de MAGNETRON.":
    "Enable DHT discovery under Sources. It runs while MAGNETRON is open and stops when you close it. BitTorrent and eD2k downloads are built in.",
  "Incluye nuevas fuentes públicas, prioriza las que responden y reintenta las que fallan. El motor se actualiza en segundo plano y se activa al volver a abrir MAGNETRON.":
    "Includes new public sources, prioritises responsive ones and retries failures. Engine updates are prepared in the background and used on the next engine start.",
  "BitTorrent y eD2k están integrados. eD2k busca en servidores públicos y descarga dentro de MAGNETRON. El motor de Jackett está incluido en MAGNETRON. Elige los conectores públicos que quieras consultar; cada sitio puede estar caído, bloquear el acceso o requerir configuración adicional. No se sortean sus controles.":
    "BitTorrent, eD2k and the Jackett engine are built in. Sources are managed automatically. Some sites may be unavailable, block access or require configuration. Their access controls are respected.",
  "Filtrar por nombre, idioma o dominio": "Filter by name, language or domain",
  "Filtrar fuentes": "Filter sources",
  "DHT · descubrimiento adicional": "DHT · additional discovery",
  "Descubre torrents y guarda sus metadatos. La comprobación de disponibilidad al buscar funciona automáticamente, aunque este descubrimiento esté pausado.":
    "Discover torrents and save their metadata. Availability checks during search run automatically, even when discovery is paused.",
  "Pausar DHT": "Pause DHT",
  "Activar DHT": "Enable DHT",
  "Marcar todas": "Select all",
  "Desmarcar todas": "Deselect all",
  "Guardar selección": "Save selection",
  "Red descentralizada eD2k": "Decentralised eD2k network",
  "Servidores eD2k públicos": "Public eD2k servers",
  Todas: "All",
  Directa: "Direct",
  "Índice DHT": "DHT index",
  "Arrancando el motor integrado y leyendo conectores…":
    "Starting the built-in engine and loading sources…",
  "Los cambios se aplican a la siguiente búsqueda.":
    "Changes apply to the next search.",
  "Fuentes guardadas. Se aplicarán a la siguiente búsqueda.":
    "Sources saved. Changes apply to your next search.",
  "Registros guardados": "Saved records",
  "Nodos que respondieron": "Responding nodes",
  "Hashes descubiertos": "Discovered hashes",
  "Metadatos recuperados": "Retrieved metadata",
  "Intentos sin metadatos": "Attempts without metadata",
  Nombre: "Name",
  Red: "Network",
  Tamaño: "Size",
  Progreso: "Progress",
  Estado: "Status",
  "↓ Bajada": "↓ Download",
  "↑ Subida": "↑ Upload",
  "Tiempo restante": "Time left",
  Acciones: "Actions",
  "Descargas BitTorrent y eD2k": "BitTorrent and eD2k downloads",
  "Los archivos se guardan en la carpeta elegida. Durante la descarga se intercambian piezas. Al completar se liberan los archivos; cancelar conserva los parciales.":
    "Files are saved in your chosen folder. Pieces are exchanged during downloads. Completed files are released; cancelling keeps partial files.",
  "Todavía no hay descargas. Busca un archivo y pulsa Descargar.":
    "No downloads yet. Search for a file or paste a link to get started.",
  "Buscando fuentes": "Finding sources",
  Comprobando: "Checking",
  Descargando: "Downloading",
  "Esperando fuentes o turno": "Waiting for sources or a queue slot",
  "Verificando archivo": "Verifying file",
  Pausada: "Paused",
  Cancelada: "Cancelled",
  Completada: "Completed",
  Pausar: "Pause",
  Reintentar: "Retry",
  Reanudar: "Resume",
  Cancelar: "Cancel",
  Carpeta: "Folder",
  "Abrir carpeta": "Open folder",
  "Quitar del listado (conservar archivos)": "Remove from list (keep files)",
  "Eliminar también los archivos…": "Delete files too…",
  "Eliminar archivos": "Delete files",
  Conservar: "Keep files",
  "Se eliminarán definitivamente los archivos de esta descarga y sus subcarpetas vacías. Se conservarán la carpeta de destino y los demás archivos.":
    "This permanently deletes this download's files and its empty subfolders. Your destination folder and other files will be kept.",
  "Descarga y archivos eliminados": "Download and files deleted",
  "Descarga quitada del listado. Archivos conservados.":
    "Download removed from the list. Files kept.",
  "Elige una carpeta para iniciar las descargas.":
    "Choose a folder to start downloading.",
  "Preparando descarga…": "Preparing download…",
  "Preparando…": "Preparing…",
  Descargar: "Download",
  Copiar: "Copy",
  "Enlace copiado": "Link copied",
  "No se pudo acceder al portapapeles": "Could not access the clipboard",
  "Fecha desconocida": "Unknown date",
  "BitTorrent · Fuente completa accesible":
    "BitTorrent · Reachable complete source",
  Reportada: "Reported",
  "Sin comprobar": "Unchecked",
  "Buscando fuentes completas accesibles…":
    "Looking for reachable complete sources…",
  "Ningún resultado con estos filtros.": "No results match these filters.",
  "Búsqueda finalizada con errores de fuente.":
    "Search finished with source errors.",
  "No se encontraron fuentes completas accesibles.":
    "No reachable complete sources found.",
  "Solo aparecen archivos que superan la comprobación directa. Puede tardar unos minutos.":
    "Only files that pass a direct availability check appear. This may take a few minutes.",
  "Puedes consultar la caché disponible o volver a intentarlo.":
    "You can search cached results or try again.",
  "No pudimos comprobar una fuente completa desde tu equipo. Eso no significa que el archivo no exista; puedes probar más tarde o cambiar la búsqueda.":
    "We could not verify a complete source from your device. The file may still exist; try again later or change your search.",
  "BitTorrent comprobado en directo · eD2k con fuentes completas reportadas.":
    "BitTorrent checked live · eD2k with reported complete sources.",
  "Buscando y comprobando fuentes completas…":
    "Searching and checking complete sources…",
  "No se pudo buscar": "Search failed",
  "La búsqueda admite hasta 200 caracteres":
    "Search text can contain up to 200 characters",
  "Ajustes guardados": "Settings saved",
  "Caché e historial eliminados.": "Cache and history cleared.",
  "Datos locales eliminados.": "Local data cleared.",
  "¿Borrar la caché y todo el historial local? Las preferencias se conservan.":
    "Clear the cache and all local search history? Preferences will be kept.",
  "Disponible en escritorio": "Available in the desktop app",
  "Disponible en la aplicación de escritorio": "Available in the desktop app",
  "Disponible en MAGNETRON de escritorio": "Available in MAGNETRON for desktop",
  "Abre MAGNETRON de escritorio para consultar las fuentes reales.":
    "Open MAGNETRON for desktop to search real sources.",
  "Los ajustes persistentes requieren la aplicación de escritorio.":
    "Persistent settings require the desktop app.",
  "Pega como máximo 100 enlaces": "Paste up to 100 links",
  "Se necesita un enlace eD2k de archivo": "An eD2k file link is required",
  "Extensión de enlace eD2k no admitida": "Unsupported eD2k link extension",
  "Pega enlaces ed2k:// o magnet:, uno por línea":
    "Paste ed2k:// or magnet: links, one per line",
  "Pega un enlace eD2k o magnet": "Paste an eD2k or magnet link",
  "Enlace eD2k inválido": "Invalid eD2k link",
  "Solo se permiten enlaces eD2k de fichero":
    "Only eD2k file links are supported",
  "Elige una carpeta existente": "Choose an existing folder",
  "Elige primero la carpeta de descarga en Ajustes":
    "Choose a download folder in Settings first",
  "Descarga no encontrada": "Download not found",
  "La descarga ya ha terminado": "The download has already finished",
  "La descarga ya está en curso": "The download is already running",
  "La carpeta de descarga cambió de ubicación; no se borra ningún archivo":
    "The download folder has moved; no files were deleted",
  "La carpeta de descarga ya no está disponible":
    "The download folder is no longer available",
  "El archivo terminado está fuera de la carpeta privada":
    "The completed file is outside the private staging folder",
  "El motor no puede continuar; comprueba el espacio disponible y reanuda":
    "The engine cannot continue; check available space and resume",
  "Se necesita un magnet BitTorrent v1":
    "A BitTorrent v1 magnet link is required",
  "Hash inválido": "Invalid hash",
  "Acción desconocida": "Unknown action",
  "No llegaron los metadatos. Puedes reintentar la descarga.":
    "Metadata did not arrive. You can retry the download.",
  "La fuente tardó demasiado": "The source took too long",
  "Fuente pública no compatible": "Unsupported public source",
  "El motor no está disponible": "The engine is unavailable",
  "La fuente no respondió dentro del plazo":
    "The source did not respond in time",
  "No se pudo activar la fuente": "Could not enable the source",
  "Respuesta interrumpida": "Response interrupted",
  "Fuentes completas comunicadas": "Reported complete sources",
};
const patterns: Array<[RegExp, (...args: string[]) => string]> = [
  [/^(\d+) guardados$/, (_, n) => `${n} saved`],
  [
    /^Comprobando BitTorrent · (\d+) revisados · (\d+) accesibles$/,
    (_, a, b) => `Checking BitTorrent · ${a} checked · ${b} reachable`,
  ],
  [
    /^(Todas|Directa) · (.*)$/,
    (_, kind, rest) =>
      `${kind === "Todas" ? "All" : "Direct"} · ${translate(rest)}`,
  ],
  [
    /^(Resultados parciales|Comprobación terminada) · (.*)$/,
    (_, label, time) => `${translate(label)} · ${time}`,
  ],
  [/^(\d+) archivos · (.*)$/, (_, n, rest) => `${n} files · ${rest}`],
  [
    /^(\d+) (?:fuente|fuentes) · Ver detalles$/,
    (_, n) => `${n} ${n === "1" ? "source" : "sources"} · Details`,
  ],
  [/^(\d+) archivos indexados$/, (_, n) => `${n} indexed files`],
  [
    /^eD2k · (\d+) completas \(red\)$/,
    (_, n) => `eD2k · ${n} complete (network)`,
  ],
  [
    /^(\d+) (?:coincidencia|coincidencias)( de \d+)? · (.*)$/,
    (_, n, total, q) =>
      `${n} ${n === "1" ? "match" : "matches"}${total ? total.replace(" de ", " of ") : ""} · ${q}`,
  ],
  [
    /^(\d+) fuentes disponibles · (\d+) seleccionadas · (\d+) visibles$/,
    (_, a, b, c) => `${a} sources available · ${b} selected · ${c} shown`,
  ],
  [
    /^(\d+) (?:enlace añadido|enlaces añadidos) a Descargas\.$/,
    (_, n) => `${n} ${n === "1" ? "link added" : "links added"} to Downloads.`,
  ],
  [
    /^(\d+) añadidos; (\d+) pendientes\. (.*)$/,
    (_, a, b, e) => `${a} added; ${b} pending. ${translate(e)}`,
  ],
  [/^Progreso de (.*)$/, (_, name) => `Progress of ${name}`],
  [/^Acciones de (.*)$/, (_, name) => `Actions for ${name}`],
  [
    /^¿Eliminar «(.*)» y sus archivos\?$/,
    (_, name) => `Delete “${name}” and its files?`,
  ],
  [/^Descubierto: (.*)$/, (_, time) => `Discovered: ${time}`],
  [
    /^Semillas reportadas por la fuente: (.*)\. No es una medición propia\.$/,
    (_, n) => `Seeds reported by the source: ${n}. Not independently measured.`,
  ],
  [
    /^Fuentes completas reportadas por la red: (.*?)\. Indica fuentes completas;.*$/,
    (_, time) =>
      `Complete sources reported by the network: ${time}. Their connectivity and download queue have not been checked. MAGNETRON manages the download.`,
  ],
  [
    /^Comprobación directa: (.*?)\. Un peer anuncia.*$/,
    (_, time) =>
      `Direct check: ${time}. A peer advertises all pieces and accepts requests. Its metadata matches the hash. Connectivity may change; file content has not been downloaded.`,
  ],
  [
    /^Ya existe (.*?)\. No se sobrescribe; cambia su nombre o elige otra carpeta$/,
    (_, file) =>
      `${file} already exists. Rename it or choose another folder; it will not be overwritten.`,
  ],
  [
    /^Fuente no disponible \(HTTP (\d+)\)$/,
    (_, code) => `Source unavailable (HTTP ${code})`,
  ],
  [
    /^No se pudo inicializar: (.*)$/,
    (_, error) => `Could not initialise: ${translate(error)}`,
  ],
];
export function translate(text: string): string {
  if (language === "es") return text;
  const clean = text.trim();
  if (pairs[clean]) return text.replace(clean, pairs[clean]);
  const categories = clean.split(" · ");
  if (
    categories.length > 1 &&
    categories.every((c) =>
      [
        "ISO / imágenes de disco",
        "Vídeo",
        "Documentos",
        "Audio",
        "Imágenes",
        "Comprimidos",
        "Programas / juegos",
        "Otros ficheros",
        "Sin clasificar",
      ].includes(c),
    )
  )
    return categories.map((c) => pairs[c] ?? c).join(" · ");
  for (const [pattern, replace] of patterns) {
    const match = clean.match(pattern);
    if (match) return replace(...match);
  }
  return text;
}
const system = () =>
  typeof matchMedia === "function" &&
  matchMedia("(prefers-color-scheme: dark)").matches;
function applyAppearance() {
  document.documentElement.dataset.theme =
    appearance === "system" ? (system() ? "dark" : "light") : appearance;
  document.documentElement.style.colorScheme =
    document.documentElement.dataset.theme;
}
export function setAppearance(value: Appearance) {
  appearance = value;
  persist("magnetron.appearance", value);
  applyAppearance();
  document.dispatchEvent(
    new CustomEvent("magnetron:appearance", { detail: value }),
  );
}
export function setLanguage(value: Language) {
  language = value;
  persist("magnetron.language", value);
  document.documentElement.lang = value;
  localize(document.body);
  document.dispatchEvent(new CustomEvent("magnetron:language"));
}
// Bind only UI copy. Filenames, queries and source-supplied content are explicitly excluded.
const textBindings = new WeakMap<Node, { source: string; last: string }>();
const attributeBindings = new WeakMap<
  Element,
  Map<string, { source: string; last: string }>
>();
function ignored(element: Element | null) {
  return !!element?.closest("[data-no-i18n],script,style,textarea,input,code");
}
export function localize(root: Node) {
  if (root.nodeType === Node.TEXT_NODE) {
    if (ignored(root.parentElement)) return;
    const current = root.textContent ?? "";
    let binding = textBindings.get(root);
    if (!binding || current !== binding.last) {
      binding = { source: current, last: current };
      textBindings.set(root, binding);
    }
    const next = translate(binding.source);
    binding.last = next;
    if (current !== next) root.textContent = next;
    return;
  }
  if (root instanceof Element) {
    if (root.closest("[data-no-i18n],script,style,code")) return;
    for (const name of ["title", "placeholder", "aria-label"]) {
      const current = root.getAttribute(name);
      if (current === null) continue;
      let attrs = attributeBindings.get(root);
      if (!attrs) {
        attrs = new Map();
        attributeBindings.set(root, attrs);
      }
      let binding = attrs.get(name);
      if (!binding || current !== binding.last) {
        binding = { source: current, last: current };
        attrs.set(name, binding);
      }
      const next = translate(binding.source);
      binding.last = next;
      if (next !== current) root.setAttribute(name, next);
    }
  }
  for (const child of Array.from(root.childNodes)) localize(child);
}
export function setupLanguageAndAppearance() {
  document.documentElement.lang = language;
  applyAppearance();
  localize(document.body);
  const observer = new MutationObserver((records) => {
    for (const record of records) {
      if (record.type === "childList") {
        for (const node of record.addedNodes) localize(node);
      } else localize(record.target);
    }
  });
  observer.observe(document.body, {
    subtree: true,
    childList: true,
    characterData: true,
    attributes: true,
    attributeFilter: ["title", "placeholder", "aria-label"],
  });
  if (typeof matchMedia === "function")
    matchMedia("(prefers-color-scheme: dark)").addEventListener(
      "change",
      () => {
        if (appearance === "system") applyAppearance();
      },
    );
  return observer;
}
export const getAppearance = () => appearance;
