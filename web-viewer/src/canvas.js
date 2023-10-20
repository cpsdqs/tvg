import { useEffect, useMemo, useRef, useState } from 'preact/hooks';
import { html } from 'htm/preact';

class Palette {
    colors = new Map();
    constructor(data) {
        if (!data) return;

        for (const color of data.colors) {
            const entry = {};
            for (const tag of color.tags) {
                if (tag.type === 'color_id') entry.id = tag.content;
                else if (tag.type === 'color_rgba') entry.rgba = tag.content;
            }
            if (entry.id) this.colors.set(entry.id.id, entry);
        }
    }

    getCssColor(color) {
        if (!color) return null;
        const entry = this.colors.get(color);
        if (entry.rgba) {
            const [r, g, b, a] = entry.rgba;
            return `rgba(${r}, ${g}, ${b}, ${a / 255})`;
        }
    }
}

const INITIAL_SCALE = 0.1;
export function CanvasView({ file }) {
    const viewNode = useRef();
    const [viewCenter, setViewCenter] = useState([0, 0]);
    const [viewOffset, setViewOffset] = useState([0, 0]);
    const [viewScale, setViewScale] = useState(INITIAL_SCALE);

    const resetCenter = () => {
        if (viewNode) setViewCenter([viewNode.current.offsetWidth / 2, viewNode.current.offsetHeight / 2]);
    };

    useEffect(resetCenter, []);
    useEffect(() => {
        window.addEventListener('resize', resetCenter);
        return () => window.removeEventListener('resize', resetCenter);
    }, [resetCenter, setViewCenter]);

    const onWheel = (e) => {
        e.preventDefault();

        if (e.ctrlKey) {
            const newScale = viewScale * (1 - e.deltaY / 50);
            const newOffset = [
                viewOffset[0] * newScale / viewScale,
                viewOffset[1] * newScale / viewScale,
            ];
            setViewScale(newScale);
            setViewOffset(newOffset);
        } else {
            setViewOffset([viewOffset[0] + e.deltaX, viewOffset[1] + e.deltaY]);
        }
    };

    const resetView = () => {
        setViewOffset([0, 0]);
        setViewScale(INITIAL_SCALE);
    };

    const main = file.find(item => item.type === 'main');
    if (!main) return html`<div class="canvas-view">no data</div>`;

    const layers = useMemo(() => [
        main.content.find(item => item.type === 'layer_underlay')?.content,
        main.content.find(item => item.type === 'layer_color')?.content,
        main.content.find(item => item.type === 'layer_line')?.content,
        main.content.find(item => item.type === 'layer_overlay')?.content,
    ], [file]);

    const palette = useMemo(() => new Palette(main.content.find(item => item.type === 'palette')?.content), [file]);

    return html`
        <div
            class="canvas-view"
            ref=${viewNode}
            onWheel=${onWheel}
        >
            <svg
                class="rendered"
            >
                <g style=${{
                    transform: `translate(${
                        viewCenter[0] - viewOffset[0]
                    }px, ${
                        viewCenter[1] - viewOffset[1]
                    }px) scale(${viewScale})`,
                }}>
                    <${AxesAndFrame} />
                    <${DrawingLayer} type="underlay" layer="${layers[0]}" palette=${palette} />
                    <${DrawingLayer} type="color" layer="${layers[1]}" palette=${palette} />
                    <${DrawingLayer} type="line" layer="${layers[2]}" palette=${palette} />
                    <${DrawingLayer} type="overlay" layer="${layers[3]}" palette=${palette} />
                </g>
            </svg>
            <div class="canvas-controls">
                <button onClick=${resetView}>reset view</button>
            </div>
        </div>
    `;
}

function AxesAndFrame() {
    // eyeballed 16:9 frame from harmony
    return html`
        <g>
            <line x1="-3333" x2="3333" y1="0" y2="0" stroke="#f00" stroke-width="5" />
            <line y1="-1875" y2="1875" x1="0" x2="0" stroke="#0f0" stroke-width="5" />
            <rect x="-3333" y="-1875" width="6666" height="3750" fill="none" stroke="#0007" stroke-width="5" />
        </g>
    `;
}

function DrawingLayer({ type, layer, palette }) {
    if (!layer || layer.type !== 'vector') return null;

    const items = [];
    for (let i = 0; i < layer.content.length; i++) {
        const shape = layer.content[i];
        if (shape.type === 'fill') {
            items.push(html`<${FillShape} key=${i} shape=${shape} palette=${palette} />`);
        } else if (shape.type === 'stroke') {
            items.push(html`<${StrokeShape} key=${i} shape=${shape} palette=${palette} />`);
        }
    }

    return html`
        <g class="layer-${type}" fill-rule="evenodd">
            ${items}
        </g>
    `;
}

function mapCoord([x, y]) {
    return [x, -y];
}

function pathSegmentsToSvgData(segments) {
    let d = '';
    for (const segment of segments) {
        if (segment.type === 'line') {
            if (d) d += `L${mapCoord(segment.content)} `;
            else d += `M${mapCoord(segment.content)} `;
        } else if (segment.type === 'cubic') {
            d += `C${mapCoord(segment.content[0])} ${mapCoord(segment.content[1])} ${mapCoord(segment.content[2])} `;
        } else {
            throw new Error(`unknown segment type ${segment.type}`);
        }
    }
    return d;
}

function FillShape({ shape, palette }) {
    let color = null;
    let d = '';
    for (const component of shape.components) {
        for (const tag of component.tags) {
            if (tag.type === 'info') {
                if (tag.content.color_id) color = tag.content.color_id;
            } else if (tag.type === 'path') {
                d += pathSegmentsToSvgData(tag.content.segments);
            }
        }
    }

    let fill = palette.getCssColor(color);
    return html`<path d=${d} fill=${fill} />`;
}

function createThicknessPaths(data) {
    let leftD = '';
    let rightD = '';

    for (let i = 0; i < data.length - 1; i++) {
        const curr = data[i];
        const next = data[i + 1];

        if (!leftD) {
            leftD += 'M' + [curr.loc, curr.left.offset];
            rightD += 'M' + [curr.loc, curr.right.offset];
        }
        leftD += ' C' + [curr.loc + curr.left.ctrl_fwd[0], curr.left.ctrl_fwd[1]];
        rightD += ' C' + [curr.loc + curr.right.ctrl_fwd[0], curr.right.ctrl_fwd[1]];
        leftD += ' ' + [next.loc - next.left.ctrl_back[0], next.left.ctrl_back[1]] + ' ';
        rightD += ' ' + [next.loc - next.right.ctrl_back[0], next.right.ctrl_back[1]] + ' ';
        leftD += ' ' + [next.loc, next.left.offset];
        rightD += ' ' + [next.loc, next.right.offset];
    }

    const left = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    left.setAttribute('d', leftD);
    const right = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    right.setAttribute('d', rightD);

    const leftLength = left.getTotalLength();
    const rightLength = right.getTotalLength();

    return {
        data,
        left,
        right,
        leftLength,
        rightLength,
    };
}

function StrokeShape({ shape, palette }) {
    return useMemo(() => {
        let color = null;
        let thickness = null;

        let leftPoints = [];
        let rightPoints = [];

        let firstPoint = [0, 0];
        let firstTangent = [0, 0];
        let hasFirstPoint = false;
        let lastPoint = [0, 0];
        let lastTangent = [0, 0];

        const normalize = v => {
            const len = Math.hypot(v[0], v[1]);
            if (len > 0) {
                v[0] /= len;
                v[1] /= len;
            }
        };

        for (const component of shape.components) {
            let d = '';
            let thicknessDomain = [0, 1];

            for (const tag of component.tags) {
                if (tag.type === 'info') {
                    if (tag.content.color_id) color = tag.content.color_id;
                } else if (tag.type === 'path') {
                    d += pathSegmentsToSvgData(tag.content.segments);
                } else if (tag.type === 'thickness') {
                    if (tag.content.definition) thickness = createThicknessPaths(tag.content.definition);
                    thicknessDomain = tag.content.domain;
                }
            }

            if (!thickness) continue;

            const centerline = document.createElementNS('http://www.w3.org/2000/svg', 'path');
            centerline.setAttribute('d', d);
            const centerlineLength = centerline.getTotalLength();

            if (!hasFirstPoint) {
                hasFirstPoint = true;
                const p = centerline.getPointAtLength(0);
                const p1 = centerline.getPointAtLength(0.1);
                firstTangent = [p1.x - p.x, p1.y - p.y];
                normalize(firstTangent);
                firstPoint = [p.x, p.y];
            }
            {
                const p = centerline.getPointAtLength(centerlineLength - 0.1);
                const p1 = centerline.getPointAtLength(centerlineLength);
                lastTangent = [p1.x - p.x, p1.y - p.y];
                normalize(lastTangent);
                lastPoint = [p1.x, p1.y];
            }

            const toCenterlineT = t => {
                return (t - thicknessDomain[0]) / (thicknessDomain[1] - thicknessDomain[0]);
            };

            const tIncrement = Math.min(1, 1 / centerlineLength);

            let started = false;
            for (let t = 0; t <= 1; t += tIncrement) {
                const leftPoint = thickness.left.getPointAtLength(t * thickness.leftLength);
                if (leftPoint.x >= thicknessDomain[0]) started = true;
                if (leftPoint.x >= thicknessDomain[1]) break;
                if (!started) continue;

                const leftT = toCenterlineT(leftPoint.x) * centerlineLength;
                const leftTP = centerline.getPointAtLength(leftT);
                const leftTP1 = centerline.getPointAtLength(leftT + 0.1);
                const leftTan = [leftTP1.x - leftTP.x, leftTP1.y - leftTP.y];
                normalize(leftTan);
                const leftDir = [leftTan[1], -leftTan[0]];

                const [leftX, leftY] = [
                    leftTP.x + leftDir[0] * leftPoint.y,
                    leftTP.y + leftDir[1] * leftPoint.y,
                ];
                leftPoints.push([leftX, leftY]);
            }

            started = false;
            for (let t = 0; t <= 1; t += tIncrement) {
                const rightPoint = thickness.right.getPointAtLength(t * thickness.rightLength);
                if (rightPoint.x >= thicknessDomain[0]) started = true;
                if (rightPoint.x >= thicknessDomain[1]) break;
                if (!started) continue;

                const rightT = toCenterlineT(rightPoint.x) * centerlineLength;
                const rightTP = centerline.getPointAtLength(rightT);
                const rightTP1 = centerline.getPointAtLength(rightT + 0.1);
                const rightTan = [rightTP1.x - rightTP.x, rightTP1.y - rightTP.y];
                normalize(rightTan);
                const rightDir = [-rightTan[1], rightTan[0]];

                const [rightX, rightY] = [
                    rightTP.x + rightDir[0] * rightPoint.y,
                    rightTP.y + rightDir[1] * rightPoint.y,
                ];
                rightPoints.push([rightX, rightY]);
            }
        }

        let d = '';
        for (const p of leftPoints) {
            if (!d) d += 'M';
            else d += ' L';
            d += p.toString();
        }

        // create basic round caps
        // some day we may know where these are stored in the TVG data
        const endCapOffset = 1.33;

        // end cap
        if (thickness) {
            const lastTP = thickness.data.at(-1);

            if (lastTP) {
                const leftDir = [lastTangent[1], -lastTangent[0]];
                const rightDir = [-lastTangent[1], lastTangent[0]];

                const startLeft = [
                    lastPoint[0] + leftDir[0] * lastTP.left.offset,
                    lastPoint[1] + leftDir[1] * lastTP.left.offset,
                ];
                const offsetLeft = endCapOffset * lastTP.left.offset;
                const offsetRight = endCapOffset * lastTP.right.offset;
                const fwdLeft = [
                    lastPoint[0] + lastTangent[0] * offsetLeft + leftDir[0] * lastTP.left.ctrl_fwd[1],
                    lastPoint[1] + lastTangent[1] * offsetLeft + leftDir[1] * lastTP.left.ctrl_fwd[1],
                ];
                const fwdRight = [
                    lastPoint[0] + lastTangent[0] * offsetRight + rightDir[0] * lastTP.right.ctrl_fwd[1],
                    lastPoint[1] + lastTangent[1] * offsetRight + rightDir[1] * lastTP.right.ctrl_fwd[1],
                ];
                const endRight = [
                    lastPoint[0] + rightDir[0] * lastTP.right.offset,
                    lastPoint[1] + rightDir[1] * lastTP.right.offset,
                ];

                d += ` L${startLeft} C${fwdLeft} ${fwdRight} ${endRight}`;
            }
        }

        for (const p of rightPoints.reverse()) {
            d += ' L' + p.toString();
        }

        // start cap
        if (thickness) {
            const firstTP = thickness.data[0];

            if (firstTP) {
                const leftDir = [firstTangent[1], -firstTangent[0]];
                const rightDir = [-firstTangent[1], firstTangent[0]];

                const startRight = [
                    firstPoint[0] + rightDir[0] * firstTP.right.offset,
                    firstPoint[1] + rightDir[1] * firstTP.right.offset,
                ];
                const offsetLeft = endCapOffset * firstTP.left.offset;
                const offsetRight = endCapOffset * firstTP.right.offset;
                const backRight = [
                    firstPoint[0] - firstTangent[0] * offsetRight + rightDir[0] * firstTP.right.ctrl_back[1],
                    firstPoint[1] - firstTangent[1] * offsetRight + rightDir[1] * firstTP.right.ctrl_back[1],
                ];
                const backLeft = [
                    firstPoint[0] - firstTangent[0] * offsetLeft + leftDir[0] * firstTP.left.ctrl_back[1],
                    firstPoint[1] - firstTangent[1] * offsetLeft + leftDir[1] * firstTP.left.ctrl_back[1],
                ];
                const endLeft = [
                    firstPoint[0] + leftDir[0] * firstTP.left.offset,
                    firstPoint[1] + leftDir[1] * firstTP.left.offset,
                ];

                d += ` L${startRight} C${backRight} ${backLeft} ${endLeft}`;
            }
        }

        let fill = palette.getCssColor(color);
        return html`<path d=${d} fill=${fill} />`;
    }, [shape, palette]);
}
