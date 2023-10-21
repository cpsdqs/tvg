import { useEffect, useMemo, useState } from 'preact/hooks';
import { html } from 'htm/preact';

export function FileMetadata({ file }) {
    return html`
        <div class="file-metadata">
            <div class="metadata-item">
                <div class="item-title">palette</div>
                <${FilePalette} file=${file} />
            </div>
            <div class="metadata-item">
                <div class="item-title">certificate</div>
                <${FileCertificate} file=${file} />
            </div>
            <div class="metadata-item">
                <div class="item-title">identity</div>
                <${FileIdentity} file=${file} />
            </div>
            <div class="metadata-item">
                <div class="item-title">signature</div>
                <${FileSignature} file=${file} />
            </div>
        </div>
    `;
}

function FileCertificate({ file }) {
    const cert = useMemo(() => file.find(item => item.type === 'certificate'), [file]);

    let dataUrl = null;
    if (cert) {
        const match = cert.content.match(/^-----BEGIN CERTIFICATE-----\n([\s\S]+?)\n-----END CERTIFICATE-----/);
        if (match) {
            const base64 = match[1].replace(/\n/g, '');
            dataUrl = `data:application/octet-stream;base64,${base64}`;
        }
    }

    const [binaryData, setBinaryData] = useState(null);
    useEffect(() => {
        setBinaryData(null);
        if (dataUrl) {
            fetch(dataUrl).then(res => res.arrayBuffer()).then(res => setBinaryData(new Uint8Array(res)));
        }
    }, [dataUrl]);

    if (binaryData) {
        return html`
            <details class="file-certificate">
                <summary class="header">certificate data</summary>
                <pre>${[...binaryData].map(i => i.toString(16).padStart(2, '0')).join(' ')}</pre>
                <pre>${new TextDecoder().decode(binaryData)}</pre>
            </details>
        `;
    } else if (cert) {
        return html`
            <details class="file-certificate">
                <summary class="header">certificate data</summary>
                <pre>${cert.content}</pre>
            </details>
        `;
    } else {
        return html`<div class="file-certificate">no certificate</div>`;
    }
}

function FileIdentity({ file }) {
    const identity = useMemo(() =>
        file
            .find(item => item.type === 'main')
            ?.content
            ?.find(item => item.type === 'identity'),
        [file],
    );

    if (identity) {
        return html`
            <div class="file-identity">
                <div class="entry">
                    <div class="label">Device</div>
                    <div class="value spoilered">
                        ${identity.content.device}
                    </div>
                </div>
                <div class="entry">
                    <div class="label">Software</div>
                    <div class="value">
                        ${identity.content.software_name}
                    </div>
                </div>
            </div>
        `;
    } else {
        return html`<div class="file-identity">no identity</div>`;
    }
}

function FileSignature({ file }) {
    const sign = useMemo(() => file.find(item => item.type === 'signature'), [file]);

    if (sign) {
        return html`
            <div class="file-signature">
                <pre>${sign.content.map(i => i.toString(16).padStart(2, '0')).join(' ')}</pre>
            </div>
        `;
    } else {
        return html`<div class="file-signature">no signature</div>`;
    }
}

function FilePalette({ file }) {
    const palette = useMemo(() =>
        file
            .find(item => item.type === 'main')
            ?.content
            ?.find(item => item.type === 'palette'),
        [file],
    );

    if (palette) {
        return html`
            <div class="file-palette">
                <ul class="color-entries">
                    ${palette.content.colors.map(color => {
                        const id = color.tags.find(t => t.type === 'color_id');
                        const rgba = color.tags.find(t => t.type === 'color_rgba');
                        
                        let value = null;
                        if (rgba) {
                            const [r, g, b, a] = rgba.content;
                            let isLight = (r * 0.2 + g * 0.7 + b * 0.1) > 127;

                            value = html`
                                <div class="color-value">
                                    <code class="rgba-value" style=${{
                                        background: `rgba(${r}, ${g}, ${b}, ${a / 255})`,
                                        color: isLight ? 'black' : 'white',
                                    }}>
                                        ${r} ${g} ${b} ${a}
                                    </code>
                                </div>
                            `;
                        }

                        return html`
                            <li class="color-entry">
                                <div class="color-id">
                                    <code class="color-id-value">${id?.content?.id?.toString(16)}</code>
                                    <span class="color-label">
                                        <span class="color-palette">${id?.content?.palette || '?'}</span>
                                        <span class="color-name">${id?.content?.name || '?'}</span>
                                    </span>
                                </div>
                                ${value}
                            </li>
                        `;
                    })}
                </ul>
            </div>
        `;
    } else {
        return html`<div class="file-palette">no palette</div>`;
    }
}
