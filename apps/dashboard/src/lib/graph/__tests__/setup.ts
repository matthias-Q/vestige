/**
 * Test setup: minimal DOM stubs for canvas-based text rendering.
 */
import { vi } from 'vitest';

// Minimal canvas 2D context mock
const mockContext2D = {
	clearRect: vi.fn(),
	fillText: vi.fn(),
	measureText: vi.fn(() => ({ width: 100 })),
	font: '',
	textAlign: '',
	textBaseline: '',
	fillStyle: '',
	shadowColor: '',
	shadowBlur: 0,
	shadowOffsetX: 0,
	shadowOffsetY: 0,
};

// Minimal canvas element mock
const mockCanvas = {
	width: 512,
	height: 64,
	getContext: vi.fn(() => mockContext2D),
	toDataURL: vi.fn(() => 'data:image/png;base64,'),
};

// Stub document.createElement for canvas
if (typeof globalThis.document === 'undefined') {
	(globalThis as any).document = {
		createElement: vi.fn((tag: string) => {
			if (tag === 'canvas') return { ...mockCanvas };
			return {};
		}),
	};
}
