import { render } from 'preact';
import { useMemo, useRef, useState } from 'preact/hooks';
import { html } from 'htm/preact';
import { decode } from 'msgpackr';
import initWasm, { readTVG } from '../tvg-wasm-out/tvg_wasm.js';
import { FileMetadata } from './metadata.js';
import { CanvasView } from './canvas.js';
import { LayerDataViewer } from './layer-data.js';
import { selectionContext, shapeVizContext } from './ctx.js';

function LoadFile({ onLoad }) {
    const fileInput = useRef();
    const [isDraggingOver, setDraggingOver] = useState(false);

    const showFileSelector = () => fileInput.current.click();

    const onInputChange = (e) => {
        const file = e.target.files[0];
        if (!file) return;

        const fr = new FileReader();
        fr.onload = () => {
            onLoad(fr.result);
            fileInput.current.value = null;
        };
        fr.onerror = () => {
            alert('error reading file');
            fileInput.current.value = null;
        };
        fr.readAsArrayBuffer(file);
    };
    const onDrop = (e) => {
        e.preventDefault();
        setDraggingOver(false);
        const file = e.dataTransfer.files[0];
        if (!file) return;

        const fr = new FileReader();
        fr.onload = () => onLoad(fr.result);
        fr.onerror = () => alert('error reading file');
        fr.readAsArrayBuffer(file);
    };

    return html`
        <div class="load-file ${isDraggingOver ? 'is-dragging-over' : ''}">
            <button
                onClick=${showFileSelector}
                onDragOver=${(e) => {
                    e.preventDefault();
                    setDraggingOver(true);
                }}
                onDragLeave=${() => {
                    setDraggingOver(false)
                }}
                onDrop=${onDrop}
            >
                load file
            </button>
            <input
                class="hidden-input"
                type="file"
                ref=${fileInput}
                onChange=${onInputChange}
            />
        </div>
    `;
}

function Tvg({ file }) {
    const [hovering, setHovering] = useState(null);
    const [selected, setSelected] = useState(null);
    const selection = useMemo(() => ({
        selected,
        hovering,
        setSelected,
        setHovering,
    }), [hovering, selected]);

    const [shapeViz, setShapeViz] = useState(null);
    const shapeVizCtx = useMemo(() => ({
        type: shapeViz?.type,
        value: shapeViz?.value,
        set: (type, value) => setShapeViz({ type, value }),
        clear: () => setShapeViz(null),
    }), [shapeViz]);

    return html`
        <div class="tvg-file">
            <${selectionContext.Provider} value=${selection}>
                <${shapeVizContext.Provider} value=${shapeVizCtx}>
                    <div class="side-panel">
                        <${LayerDataViewer} file=${file} />
                        <${FileMetadata} file=${file} />
                    </div>
                    <div class="canvas-container">
                        <${CanvasView} file=${file} />
                    </div>
                </${shapeVizContext.Provider}>
            </${selectionContext.Provider}>
        </div>
    `;
}

function Main() {
    const [tvg, setTvg] = useState(null);
    const [statusText, setStatusText] = useState(null);

    const onLoad = async (data) => {
        try {
            await initWasm();

            const t0 = Date.now();
            const fileData = readTVG(new Uint8Array(data));
            const t1 = Date.now();

            const tvg = decode(fileData);
            console.log('read file', tvg);
            setTvg(tvg);
            setStatusText(`file loaded in ${t1 - t0} ms`);
        } catch (err) {
            console.error(err);
            setStatusText(err.toString());
        }
    };

    let contents = null;
    if (tvg) {
        contents = html`<${Tvg} file=${tvg} />`;
    }

    return html`
        <div id="application">
            <div class="header">
                <${LoadFile} onLoad=${onLoad} />
                ${statusText}
            </div>
            ${contents}
        </div>
    `;
}

const container = document.createElement('div');
document.body.append(container);
render(html`<${Main} />`, container);
