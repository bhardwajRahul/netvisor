// Immediate custom action: read the per-tenant config the server encoded into the MSI
// filename and set it as installer properties, so a provisioned download is a double-click
// install with only the API key left to enter.
//
// Renaming a signed MSI does not break its Authenticode signature (the signature covers the
// file bytes, not the name), so the server ships ONE signed MSI and the download is served /
// renamed per daemon. The filename is a single compact segment:
//
//   scanopy-daemon-<base64url(querystring)>.msi
//
// where querystring is `mode=..&name=..&url=..` (values percent-encoded). Everything lives in
// ONE base64url blob rather than one `~~field=hex~~` segment per field, so the name stays
// short even as more config fields are added (per-field hex would blow past the ~255-char
// filename limit). The API KEY is deliberately NOT encoded here — a live credential must
// never sit in a filename / Downloads folder / MSI log.
//
// The decoders below are pure JScript (a streaming base64url decoder + a manual percent
// decoder) so they don't depend on engine-specific helpers like atob/decodeURIComponent
// that may be absent from the Windows Installer script host.

var B64URL_ALPHABET = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_';

function base64UrlDecode(input) {
    var out = '';
    var buffer = 0;
    var bits = 0;
    for (var i = 0; i < input.length; i++) {
        var idx = B64URL_ALPHABET.indexOf(input.charAt(i));
        if (idx < 0) {
            continue; // skip padding / stray chars
        }
        buffer = (buffer << 6) | idx;
        bits += 6;
        if (bits >= 8) {
            bits -= 8;
            out += String.fromCharCode((buffer >> bits) & 0xff);
        }
    }
    return out;
}

function percentDecode(s) {
    var out = '';
    for (var i = 0; i < s.length; i++) {
        var c = s.charAt(i);
        if (c === '%' && i + 2 < s.length) {
            out += String.fromCharCode(parseInt(s.substr(i + 1, 2), 16));
            i += 2;
        } else if (c === '+') {
            out += ' ';
        } else {
            out += c;
        }
    }
    return out;
}

function ParseFilename() {
    // Full path of the MSI currently executing (the renamed download at first install).
    var dbPath = Session.Property('OriginalDatabase');
    if (!dbPath) {
        return 1;
    }

    // Strip directory and the .msi extension, then the fixed prefix to get the blob.
    var base = dbPath.replace(/^.*[\\\/]/, '').replace(/\.msi$/i, '');
    var prefix = 'scanopy-daemon-';
    if (base.substr(0, prefix.length) !== prefix) {
        return 1; // a plain manual download — nothing encoded
    }
    var query = base64UrlDecode(base.substr(prefix.length));

    // Map query keys to installer properties.
    var map = {
        mode: 'MODE',
        name: 'DAEMONNAME',
        url: 'SERVERURL',
        addr: 'LISTENADDRESS',
        port: 'LISTENPORT',
        loglevel: 'LOGLEVEL',
        logfile: 'LOGFILE',
        interfaces: 'INTERFACES',
        acceptinvalidscan: 'ACCEPTINVALIDSCANCERTS',
        allowselfsigned: 'ALLOWSELFSIGNEDCERTS',
        heartbeat: 'HEARTBEATINTERVAL'
    };

    var pairs = query.split('&');
    for (var i = 0; i < pairs.length; i++) {
        var eq = pairs[i].indexOf('=');
        if (eq < 0) {
            continue;
        }
        var key = pairs[i].substr(0, eq);
        var prop = map[key];
        if (!prop) {
            continue;
        }
        var value = percentDecode(pairs[i].substr(eq + 1));
        if (value) {
            Session.Property(prop) = value;
        }
    }

    return 1;
}
