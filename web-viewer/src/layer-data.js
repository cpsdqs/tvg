import { useContext, useMemo, useState } from 'preact/hooks';
import { html } from 'htm/preact';
import { selectionContext, shapeVizContext } from './ctx.js';

export function LayerDataViewer({ file }) {
    const layers = useMemo(() => {
        const main = file.find(item => item.type === 'main');
        return [
            main.content.find(item => item.type === 'layer_underlay')?.content,
            main.content.find(item => item.type === 'layer_color')?.content,
            main.content.find(item => item.type === 'layer_line')?.content,
            main.content.find(item => item.type === 'layer_overlay')?.content,
        ];
    }, [file]);

    return html`
        <div class="layer-data-viewer">
            <${LayerData} type="overlay" layer=${layers[3]} />
            <${LayerData} type="line" layer=${layers[2]} />
            <${LayerData} type="color" layer=${layers[1]} />
            <${LayerData} type="underlay" layer=${layers[0]} />
        </div>
    `;
}

function LayerData({ type, layer }) {
    return html`
        <div class="layer-data">
            <div class="header">
                <span class="label">${type}</span>
                <span class="detail">${layer.type}</span>
            </div>
            <${LayerContent} type=${type} layer=${layer} />
        </div>
    `;
}

function LayerContent({ type, layer }) {
    if (layer.type === 'vector') {
        const items = [];
        for (const shape of layer.content) {
            items.push(html`<${VectorShape} shape=${shape} id=${[type, items.length].join('/')} />`);
        }

        return html`
            <div class="vector-data">
                ${items}
            </div>
        `;
    }

    return null;
}

function VectorShape({ shape, id }) {
    const sel = useContext(selectionContext);
    const [open, setOpen] = useState(false);

    const onPointerOver = () => {
        sel.setHovering(id);
    };
    const onPointerOut = () => {
        if (sel.hovering === id) sel.setHovering(null);
    };
    const onPointerDown = (e) => {
        e.stopPropagation();
        e.preventDefault();
        if (sel.selected === id) sel.setSelected(null);
        else sel.setSelected(id);
    };
    const onToggle = (e) => {
        e.preventDefault();
        setOpen(!open);
    };

    const components = [];
    if (open) {
        for (const component of shape.components) {
            components.push(html`<${VectorShapeComponent} component=${component} />`);
        }
    }

    return html`
        <details class="vector-shape" onToggle=${onToggle} open=${open}>
            <summary
                class="header ${sel.hovering === id ? 'is-hovering' : ''} ${sel.selected === id ? 'is-selected' : ''}"
                onPointerOver=${onPointerOver}
                onPointerOut=${onPointerOut}
            >
                <div class="disclosure">
                    ${open ? '▼' : '▶'}
                </div>
                <button
                    class="inner"
                    onPointerDown=${onPointerDown}
                >
                    <span class="label">${shape.type}</span>
                    <span class="detail">#c: ${shape.components.length}</span>
                </button>
            </summary>
            ${components}
        </details>
    `;
}

function VectorShapeComponent({ component }) {
    return html`
        <div class="vector-shape-component">
            ${component.tags.map((tag, i) => html`<${ComponentTag} key=${i} tag=${tag} />`)}
        </div>
    `;
}

function ComponentTag({ tag }) {
    if (tag.type === 'info') {
        return html`
            <div class="component-tag is-info">
                <span class="type">${tag.content.type}</span>
                <code class="color-id">${tag.content.color_id?.toString(16) || '—'}</code>
            </div>
        `;
    } else if (tag.type === 'path') {
        return html`
            <div class="component-tag is-path">
                ${tag.content.segments.map((segment, i) => html`
                    <${PathSegment} key=${i} segment=${segment} />
                `)}
            </div>
        `;
    }

    return null;
}

function PathSegment({ segment }) {
    if (segment.type === 'line') {
        return html`
            <div class="path-segment">
                <span class="segment-type">L</span>
                <${AbsolutePoint} point=${segment.content} />
            </div>
        `;
    } else if (segment.type === 'cubic') {
        return html`
            <div class="path-segment">
                <span class="segment-type">C</span>
                <${AbsolutePoint} point=${segment.content[0]} />
                <${AbsolutePoint} point=${segment.content[1]} />
                <${AbsolutePoint} point=${segment.content[2]} />
            </div>
        `;
    }
    return '?';
}

function AbsolutePoint({ point }) {
    const shapeViz = useContext(shapeVizContext);

    const onPointerOver = () => {
        shapeViz.set('point', point);
    };
    const isShapeViz = shapeViz.type === 'point' && shapeViz.value === point;
    const onPointerOut = () => {
        if (isShapeViz) shapeViz.clear();
    };

    return html`
        <span
            class="absolute-point ${isShapeViz ? 'is-viz' : ''}"
            onPointerOver=${onPointerOver}
            onPointerOut=${onPointerOut}
        >
            <span class="coord">${point[0]}</span>
            <span class="coord">${point[1]}</span>
        </span>
    `;
}
