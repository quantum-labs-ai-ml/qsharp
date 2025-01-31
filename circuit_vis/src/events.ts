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
    private selectedLocation: string | null;
    private selectedWire: string | null;

    constructor(container: HTMLElement, sqore: Sqore, useRefresh: () => void) {
        this.container = container;
        this.svg = container.querySelector('svg[id]') as SVGElement;
        this.dropzoneLayer = container.querySelector('.dropzone-layer') as SVGGElement;
        this.operations = sqore.circuit.operations;
        this.wireData = this._wireData();
        this.renderFn = useRefresh;
        this.selectedOperation = null;
        this.selectedLocation = null;
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
            if (ev.ctrlKey && this.selectedLocation) {
                this.container.classList.remove('moving');
                this.container.classList.add('copying');
            } else if (ev.key == 'Delete') {
                if (this.selectedLocation != null) {
                    console.log('Removing operation with location: ', this.selectedLocation);
                    this._removeOperation(this.selectedLocation);
                    this.renderFn();
                } else {
                    console.log('No operation selected');
                }
            }
        });

        document.addEventListener('keyup', (ev: KeyboardEvent) => {
            if (ev.ctrlKey && this.selectedLocation) {
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
                    this.selectedLocation = gateElem.getAttribute('data-location');
                    this.selectedOperation = this._findOperation(this.selectedLocation);
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
                const targetLoc = dropzoneElem.getAttribute('data-dropzone-location');
                const targetWire = dropzoneElem.getAttribute('data-dropzone-wire');

                // Add a new operation from the toolbox
                if (
                    this.selectedLocation == null &&
                    this.selectedOperation !== null &&
                    targetLoc !== null &&
                    targetWire !== null
                ) {
                    const newOperation = this._addOperation(this.selectedOperation, targetLoc);
                    if (newOperation != null) {
                        this._addY(targetWire, newOperation, this.wireData.length);
                    }
                    this.selectedOperation = null;
                    if (isEqual(originalOperations, this.operations) === false) this.renderFn();
                    return;
                }

                if (
                    targetLoc == null ||
                    targetWire == null ||
                    this.selectedOperation == null ||
                    this.selectedLocation == null ||
                    this.selectedWire == null
                )
                    return;

                const newSourceOperation = ev.ctrlKey
                    ? this._copyX(this.selectedLocation, targetLoc)
                    : this._moveX(this.selectedLocation, targetLoc);

                if (newSourceOperation != null) {
                    this._moveY(this.selectedWire, targetWire, newSourceOperation, this.wireData.length);
                    const parentOperation = this._findParentOperation(this.selectedLocation);
                    if (parentOperation != null) {
                        parentOperation.targets = this._targets(parentOperation);
                    }
                }

                this.selectedLocation = null;
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
    _findGateElem(hostElem: SVGElement): SVGElement | null {
        return hostElem.closest<SVGElement>('[data-location]');
    }

    /**
     * Find location of the gate surrounding a host element
     */
    _findLocation(hostElem: SVGElement) {
        const gateElem = this._findGateElem(hostElem);
        return gateElem != null ? gateElem.getAttribute('data-location') : null;
    }

    /**
     * Find the parent operation of the operation specified by location
     */
    _findParentOperation(location: string | null): Operation | null {
        if (!location) return null;

        const indexes = this._indexes(location);
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
     * Find the parent array of an operation based on its location
     */
    _findParentArray(location: string | null): Operation[] | null {
        if (!location) return null;

        const indexes = this._indexes(location);
        indexes.pop(); // The last index refers to the operation itself, remove it so that the last index instead refers to the parent operation

        let parentArray = this.operations;
        for (const index of indexes) {
            parentArray = parentArray[index].children || parentArray;
        }
        return parentArray;
    }

    /**
     * Find an operation based on its location
     */
    _findOperation(location: string | null): Operation | null {
        if (!location) return null;

        const index = this._lastIndex(location);
        const operationParent = this._findParentArray(location);

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
    _removeOperation(sourceLocation: string) {
        const sourceOperation = this._findOperation(sourceLocation);
        const sourceOperationParent = this._findParentArray(sourceLocation);

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
    _moveX = (sourceLocation: string, targetLocation: string): Operation | null => {
        const sourceOperation = this._findOperation(sourceLocation);
        if (sourceLocation === targetLocation) return sourceOperation;
        const sourceOperationParent = this._findParentArray(sourceLocation);
        const targetOperationParent = this._findParentArray(targetLocation);
        const targetLastIndex = this._lastIndex(targetLocation);

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
        if (sourceOperation.dataAttributes === undefined) {
            sourceOperation.dataAttributes = { removed: 'true' };
        } else {
            sourceOperation.dataAttributes['removed'] = 'true';
        }
        const indexToRemove = sourceOperationParent.findIndex(
            (operation) => operation.dataAttributes && operation.dataAttributes['removed'],
        );
        sourceOperationParent.splice(indexToRemove, 1);

        return newSourceOperation;
    };

    /**
     * Copy an operation horizontally
     */
    _copyX = (sourceLocation: string, targetLocation: string): Operation | null => {
        const sourceOperation = this._findOperation(sourceLocation);
        const targetOperationParent = this._findParentArray(targetLocation);
        const targetLastIndex = this._lastIndex(targetLocation);

        if (targetOperationParent == null || targetLastIndex == null || sourceOperation == null) return null;

        // Insert sourceOperation to target last index
        const newSourceOperation: Operation = JSON.parse(JSON.stringify(sourceOperation));
        targetOperationParent.splice(targetLastIndex, 0, newSourceOperation);

        return newSourceOperation;
    };

    /**
     * Copy an operation horizontally
     */
    _addOperation = (sourceOperation: Operation, targetLocation: string): Operation | null => {
        const targetOperationParent = this._findParentArray(targetLocation);
        const targetLastIndex = this._lastIndex(targetLocation);

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
     * Split location into an array of indexes
     */
    _indexes(location: string): number[] {
        return location !== '' ? location.split('-').map((segment) => parseInt(segment)) : [];
    }

    /**
     * Get the last index of location
     * i.e: location = "0-1-2", _lastIndex will return 2
     */
    _lastIndex(location: string): number | undefined {
        return this._indexes(location).pop();
    }
}

export { extensionEvents };
