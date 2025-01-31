// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

import cloneDeep from 'lodash/cloneDeep';
import { Operation } from './circuit';
import { Register } from './register';
import { Sqore } from './sqore';
import isEqual from 'lodash/isEqual';
import { defaultGateDictionary } from './panel';

const extensionEvents = (container: HTMLElement, sqore: Sqore, useRefresh: () => void): void => {
    const events = new CircuitEvents(container, sqore, useRefresh);

    events._addContextMenuEvent();
    events._addDropzoneLayerEvents();
    events._addDocumentEvents();
    events._addHostElementsEvents();
    events._addToolboxElementsEvents();
    events._addDropzoneElementsEvents();
};

class CircuitEvents {
    private container: HTMLElement;
    private svg: SVGElement;
    private dropzoneLayer: SVGGElement;
    private operations: Operation[];
    private wireData: number[];
    private renderFn: () => void;
    private selectedOperation: Operation | null;
    private selectedId: string | null;
    private selectedWire: string | null;

    constructor(container: HTMLElement, sqore: Sqore, useRefresh: () => void) {
        this.container = container;
        this.svg = container.querySelector('svg[id]') as SVGElement;
        this.dropzoneLayer = container.querySelector('.dropzone-layer') as SVGGElement;
        this.operations = sqore.circuit.operations;
        this.wireData = this._wireData();
        this.renderFn = useRefresh;
        this.selectedOperation = null;
        this.selectedId = null;
        this.selectedWire = null;
    }

    /**
     * Generate an array of y values based on circuit wires
     */
    _wireData(): number[] {
        // elems include qubit wires and lines of measure gates
        const elems = this.container.querySelectorAll<SVGGElement>('svg[id] > g:nth-child(3) > g');
        // filter out <g> elements having more than 2 elements because
        // qubit wires contain only 2 elements: <line> and <text>
        // lines of measure gates contain 4 <line> elements
        const wireElems = Array.from(elems).filter((elem) => elem.childElementCount < 3);
        const wireData = wireElems.map((wireElem) => {
            const lineElem = wireElem.children[0] as SVGLineElement;
            return Number(lineElem.getAttribute('y1'));
        });
        return wireData;
    }

    /***************************
     * Events Adding Functions *
     ***************************/

    /**
     * Add events specifically for dropzoneLayer
     */
    _addDropzoneLayerEvents() {
        this.container.addEventListener('mouseup', () => (this.dropzoneLayer.style.display = 'none'));
    }

    /**
     * Add events for document
     */
    _addDocumentEvents() {
        document.addEventListener('keydown', (ev: KeyboardEvent) => {
            if (ev.ctrlKey && this.selectedId) {
                this.container.classList.remove('moving');
                this.container.classList.add('copying');
            } else if (ev.key == 'Delete' && this.selectedId != null) {
                console.log('Removing operation with data-id: ', this.selectedId);
                this._removeOperation(this.selectedId);
                this.renderFn();
            }
        });

        document.addEventListener('keyup', (ev: KeyboardEvent) => {
            if (ev.ctrlKey && this.selectedId) {
                this.container.classList.remove('copying');
                this.container.classList.add('moving');
            }
        });

        document.addEventListener('mouseup', () => {
            this.container.classList.remove('moving', 'copying');
        });
    }

    /**
     * Disable contextmenu default behaviors
     */
    _addContextMenuEvent() {
        this.container.addEventListener('contextmenu', (ev: MouseEvent) => {
            ev.preventDefault();
        });
    }

    /**
     * Add events for circuit objects in the circuit
     */
    _addHostElementsEvents() {
        const elems = this._hostElems();
        elems.forEach((elem) => {
            elem.addEventListener('mousedown', () => {
                this.selectedWire = elem.getAttribute('data-wire');
            });

            const gateElem = this._findGateElem(elem);
            gateElem?.addEventListener('mousedown', (ev: MouseEvent) => {
                ev.stopPropagation();
                if (gateElem.getAttribute('data-expanded') !== 'true') {
                    this.selectedId = gateElem.getAttribute('data-id');
                    this.selectedOperation = this._findOperation(this.selectedId);
                    this.container.classList.add('moving');
                    this.dropzoneLayer.style.display = 'block';
                }
            });
        });
    }

    /**
     * Add events for circuit objects in the circuit
     */
    _addToolboxElementsEvents() {
        const elems = this._toolboxElems();
        elems.forEach((elem) => {
            elem.addEventListener('mousedown', () => {
                this.container.classList.add('moving');
                this.dropzoneLayer.style.display = 'block';
                const type = elem.getAttribute('data-type');
                if (type == null) return;
                this.selectedOperation = defaultGateDictionary[type];
            });

            // const gateElem = this._findGateElem(elem);
            // gateElem?.setAttribute('gate-draggable', 'true');
            // gateElem?.addEventListener('mousedown', (ev: MouseEvent) => {
            //     ev.stopPropagation();
            //     if (gateElem.getAttribute('data-expanded') !== 'true') {
            //         this.selectedId = gateElem.getAttribute('data-id');
            //         this.container.classList.add('moving');
            //         this.dropzoneLayer.style.display = 'block';
            //     }
            // });
        });
    }

    /**
     * Add events for dropzone elements
     */
    _addDropzoneElementsEvents() {
        // Dropzone element events
        const dropzoneElems = this.dropzoneLayer.querySelectorAll<SVGRectElement>('.dropzone');
        dropzoneElems.forEach((dropzoneElem) => {
            dropzoneElem.addEventListener('mouseup', (ev: MouseEvent) => {
                const originalOperations = cloneDeep(this.operations);
                const targetId = dropzoneElem.getAttribute('data-dropzone-id');
                const targetWire = dropzoneElem.getAttribute('data-dropzone-wire');

                // Add a new operation from the toolbox
                if (
                    this.selectedId == null &&
                    this.selectedOperation !== null &&
                    targetId !== null &&
                    targetWire !== null
                ) {
                    const newOperation = this._addOperation(this.selectedOperation, targetId);
                    if (newOperation != null) {
                        this._addY(targetWire, newOperation, this.wireData.length);
                    }
                    this.selectedOperation = null;
                    if (isEqual(originalOperations, this.operations) === false) this.renderFn();
                    return;
                }

                if (
                    targetId == null ||
                    targetWire == null ||
                    this.selectedOperation == null ||
                    this.selectedId == null ||
                    this.selectedWire == null
                )
                    return;

                const newSourceOperation = ev.ctrlKey
                    ? this._copyX(this.selectedId, targetId)
                    : this._moveX(this.selectedId, targetId);

                if (newSourceOperation != null) {
                    this._moveY(this.selectedWire, targetWire, newSourceOperation, this.wireData.length);
                    const parentOperation = this._findParentOperation(this.selectedId);
                    if (parentOperation != null) {
                        parentOperation.targets = this._targets(parentOperation);
                    }
                }

                this.selectedId = null;
                this.selectedOperation = null;

                if (isEqual(originalOperations, this.operations) === false) this.renderFn();
            });
        });
    }

    /**********************
     *  Finder Functions  *
     **********************/

    /**
     * Find the surrounding gate element of host element
     */
    _findGateElem(elem: SVGElement): SVGElement | null {
        return elem.closest<SVGElement>('[data-id]');
    }

    /**
     * Find data-id of host element
     */
    _findDataId(elem: SVGElement) {
        const gateElem = this._findGateElem(elem);
        return gateElem != null ? gateElem.getAttribute('data-id') : null;
    }

    /**
     * Find the parent operation of the operation specified by data-id
     */
    _findParentOperation(dataId: string | null): Operation | null {
        if (!dataId) return null;

        const indexes = this._indexes(dataId);
        indexes.pop();
        const lastIndex = indexes.pop();

        if (lastIndex == null) return null;

        let parentOperation = this.operations;
        for (const index of indexes) {
            parentOperation = parentOperation[index].children || parentOperation;
        }
        return parentOperation[lastIndex];
    }

    /**
     * Find the parent array of an operation based on its data-id
     */
    _findParentArray(dataId: string | null): Operation[] | null {
        if (!dataId) return null;

        const indexes = this._indexes(dataId);
        indexes.pop(); // The last index refers to the operation itself, remove it so that the last index instead refers to the parent operation

        let parentArray = this.operations;
        for (const index of indexes) {
            parentArray = parentArray[index].children || parentArray;
        }
        return parentArray;
    }

    /**
     * Find an operation based on its data-id from a list of operations
     */
    _findOperation(dataId: string | null): Operation | null {
        if (!dataId) return null;

        const index = this._lastIndex(dataId);
        const operationParent = this._findParentArray(dataId);

        if (
            operationParent == null || //
            index == null
        )
            return null;

        return operationParent[index];
    }

    /**************************
     *  Circuit Manipulation  *
     **************************/

    /**
     * Remove an operation
     */
    _removeOperation(sourceId: string) {
        const sourceOperation = this._findOperation(sourceId);
        const sourceOperationParent = this._findParentArray(sourceId);

        if (sourceOperation == null || sourceOperationParent == null) return null;

        // Delete sourceOperation
        if (sourceOperation.dataAttributes === undefined) {
            sourceOperation.dataAttributes = { removed: 'true' };
        } else {
            sourceOperation.dataAttributes['removed'] = 'true';
        }
        const indexToRemove = sourceOperationParent.findIndex(
            (operation) => operation.dataAttributes && operation.dataAttributes['removed'],
        );
        sourceOperationParent.splice(indexToRemove, 1);
    }

    /**
     * Move an operation horizontally
     */
    _moveX = (sourceId: string, targetId: string): Operation | null => {
        const sourceOperation = this._findOperation(sourceId);
        if (sourceId === targetId) return sourceOperation;
        const sourceOperationParent = this._findParentArray(sourceId);
        const targetOperationParent = this._findParentArray(targetId);
        const targetLastIndex = this._lastIndex(targetId);

        if (
            targetOperationParent == null ||
            targetLastIndex == null ||
            sourceOperation == null ||
            sourceOperationParent == null
        )
            return null;

        // Insert sourceOperation to target last index
        const newSourceOperation: Operation = JSON.parse(JSON.stringify(sourceOperation));
        targetOperationParent.splice(targetLastIndex, 0, newSourceOperation);

        // Delete sourceOperation
        sourceOperation.gate = 'removed';
        const indexToRemove = sourceOperationParent.findIndex((operation) => operation.gate === 'removed');
        if (indexToRemove == -1) {
            console.error("'removed' operation not found in parent array");
            return null;
        }
        sourceOperationParent.splice(indexToRemove, 1);

        return newSourceOperation;
    };

    /**
     * Copy an operation horizontally
     */
    _copyX = (sourceId: string, targetId: string): Operation | null => {
        const sourceOperation = this._findOperation(sourceId);
        const targetOperationParent = this._findParentArray(targetId);
        const targetLastIndex = this._lastIndex(targetId);

        if (targetOperationParent == null || targetLastIndex == null || sourceOperation == null) return null;

        // Insert sourceOperation to target last index
        const newSourceOperation: Operation = JSON.parse(JSON.stringify(sourceOperation));
        targetOperationParent.splice(targetLastIndex, 0, newSourceOperation);

        return newSourceOperation;
    };

    /**
     * Copy an operation horizontally
     */
    _addOperation = (sourceOperation: Operation, targetId: string): Operation | null => {
        const targetOperationParent = this._findParentArray(targetId);
        const targetLastIndex = this._lastIndex(targetId);

        if (targetOperationParent == null || targetLastIndex == null || sourceOperation == null) return null;

        // Insert sourceOperation to target last index
        const newSourceOperation: Operation = JSON.parse(JSON.stringify(sourceOperation));
        targetOperationParent.splice(targetLastIndex, 0, newSourceOperation);

        return newSourceOperation;
    };

    /**
     * Move an operation vertically by changing its controls and targets
     */
    _moveY = (sourceWire: string, targetWire: string, operation: Operation, totalWires: number): Operation => {
        if (operation.gate !== 'measure') {
            const offset = parseInt(targetWire) - parseInt(sourceWire);
            this._offsetRecursively(operation, offset, totalWires);
        }
        return operation;
    };

    /**
     * Add an operation vertically by changing its controls and targets
     */
    _addY = (targetWire: string, operation: Operation, totalWires: number): Operation => {
        if (operation.gate !== 'measure') {
            const offset = parseInt(targetWire);
            this._offsetRecursively(operation, offset, totalWires);
        }
        return operation;
    };

    /*****************
     *     Misc.     *
     *****************/

    /**
     * Get list of toolbox items
     */
    _toolboxElems(): SVGGraphicsElement[] {
        return Array.from(this.container.querySelectorAll<SVGGraphicsElement>('[toolbox-item]'));
    }

    /**
     * Get list of host elements that dropzones can be attached to
     */
    _hostElems(): SVGGraphicsElement[] {
        return Array.from(
            this.svg.querySelectorAll<SVGGraphicsElement>(
                '[class^="gate-"]:not(.gate-control, .gate-swap), .control-dot, .oplus, .cross',
            ),
        );
    }

    /**
     * Recursively change object controls and targets
     */
    _offsetRecursively(operation: Operation, wireOffset: number, totalWires: number): Operation {
        // Offset all targets by offsetY value
        if (operation.targets != null) {
            operation.targets.forEach((target) => {
                target.qId = this._circularMod(target.qId, wireOffset, totalWires);
                if (target.cId) target.cId = this._circularMod(target.cId, wireOffset, totalWires);
            });
        }

        // Offset all controls by offsetY value
        if (operation.controls != null) {
            operation.controls.forEach((control) => {
                control.qId = this._circularMod(control.qId, wireOffset, totalWires);
                if (control.cId) control.cId = this._circularMod(control.qId, wireOffset, totalWires);
            });
        }

        // Offset recursively through all children
        if (operation.children != null) {
            operation.children.forEach((child) => this._offsetRecursively(child, wireOffset, totalWires));
        }

        return operation;
    }

    /**
     * Find targets of an operation by recursively walkthrough all of its children controls and targets
     * i.e. Gate Foo contains gate H and gate RX.
     *      qIds of Gate H is 1
     *      qIds of Gate RX is 1, 2
     *      This should return [{qId: 1}, {qId: 2}]
     */
    _targets(operation: Operation): Register[] | [] {
        const _recurse = (operation: Operation) => {
            registers.push(...operation.targets);
            if (operation.controls) {
                registers.push(...operation.controls);
                // If there is more children, keep adding more to registers
                if (operation.children) {
                    for (const child of operation.children) {
                        _recurse(child);
                    }
                }
            }
        };

        const registers: Register[] = [];
        if (operation.children == null) return [];

        // Recursively walkthrough all children to populate registers
        for (const child of operation.children) {
            _recurse(child);
        }

        // Extract qIds from array of object
        // i.e. [{qId: 0}, {qId: 1}, {qId: 1}] -> [0, 1, 1]
        const qIds = registers.map((register) => register.qId);
        const uniqueQIds = Array.from(new Set(qIds));

        // Transform array of numbers into array of qId object
        // i.e. [0, 1] -> [{qId: 0}, {qId: 1}]
        return uniqueQIds.map((qId) => ({
            qId,
            type: 0,
        }));
    }

    /**
     * This modulo function always returns positive value based on total
     * i.e: value=0, offset=-1, total=4 returns 3 instead of -1
     */
    _circularMod(value: number, offset: number, total: number): number {
        return (((value + offset) % total) + total) % total;
    }

    /**
     * Split data-id into an array of indexes
     */
    _indexes(dataId: string): number[] {
        return dataId !== '' ? dataId.split('-').map((segment) => parseInt(segment)) : [];
    }

    /**
     * Get the last index of data-id
     * i.e: data-id = "0-1-2", _lastIndex will return 2
     */
    _lastIndex(dataId: string): number | undefined {
        return this._indexes(dataId).pop();
    }
}

export { extensionEvents };
