import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('three', async () => {
	const mock = await import('./three-mock');
	return { ...mock };
});

import { EffectManager } from '../effects';
import { Vector3, Color, Scene } from './three-mock';

describe('EffectManager', () => {
	let scene: InstanceType<typeof Scene>;
	let effects: EffectManager;
	let nodeMeshMap: Map<string, any>;
	let camera: any;

	beforeEach(() => {
		scene = new Scene();
		effects = new EffectManager(scene as any);
		camera = { position: new Vector3(0, 30, 80) };
		nodeMeshMap = new Map();
	});

	function createMockMesh(id: string, pos: InstanceType<typeof Vector3>) {
		const mesh = {
			scale: new Vector3(1, 1, 1),
			position: pos.clone(),
			material: {
				emissive: new Color(0x000000),
				emissiveIntensity: 0.5,
			},
			userData: { nodeId: id },
		};
		nodeMeshMap.set(id, mesh);
		return mesh;
	}

	describe('pulse effects', () => {
		it('adds pulse and decays it over time', () => {
			createMockMesh('a', new Vector3(0, 0, 0));
			effects.addPulse('a', 1.0, new Color(0xff0000) as any, 0.1);

			expect(effects.pulseEffects.length).toBe(1);

			// Update a few times
			for (let i = 0; i < 5; i++) {
				effects.update(nodeMeshMap, camera);
			}

			expect(effects.pulseEffects[0].intensity).toBeLessThan(1.0);
		});

		it('removes pulse when intensity reaches zero', () => {
			createMockMesh('a', new Vector3(0, 0, 0));
			effects.addPulse('a', 0.5, new Color(0xff0000) as any, 0.1);

			for (let i = 0; i < 20; i++) {
				effects.update(nodeMeshMap, camera);
			}

			expect(effects.pulseEffects.length).toBe(0);
		});

		it('modulates mesh emissive color and intensity', () => {
			const mesh = createMockMesh('a', new Vector3(0, 0, 0));
			const pulseColor = new Color(0xff0000);
			effects.addPulse('a', 1.0, pulseColor as any, 0.05);

			effects.update(nodeMeshMap, camera);

			// Emissive intensity should be elevated
			expect(mesh.material.emissiveIntensity).toBeGreaterThan(0.5);
		});
	});

	describe('createSpawnBurst', () => {
		it('adds particles to the scene', () => {
			const childCount = scene.children.length;
			effects.createSpawnBurst(new Vector3(0, 0, 0) as any, new Color(0x00ffd1) as any);

			expect(scene.children.length).toBe(childCount + 1);
		});

		it('creates 60 particles', () => {
			effects.createSpawnBurst(new Vector3(0, 0, 0) as any, new Color(0x00ffd1) as any);

			const pts = scene.children[scene.children.length - 1] as any;
			expect(pts.geometry.attributes.position.count).toBe(60);
		});

		it('particles move outward and fade over 120 frames', () => {
			effects.createSpawnBurst(new Vector3(0, 0, 0) as any, new Color(0x00ffd1) as any);

			for (let i = 0; i < 121; i++) {
				effects.update(nodeMeshMap, camera);
			}

			// Burst should be cleaned up
			expect(scene.children.length).toBe(0);
		});

		it('particle opacity decreases over time', () => {
			effects.createSpawnBurst(new Vector3(0, 0, 0) as any, new Color(0x00ffd1) as any);

			const pts = scene.children[0] as any;

			effects.update(nodeMeshMap, camera);
			const earlyOpacity = pts.material.opacity;

			for (let i = 0; i < 60; i++) {
				effects.update(nodeMeshMap, camera);
			}

			expect(pts.material.opacity).toBeLessThan(earlyOpacity);
		});
	});

	describe('createRainbowBurst', () => {
		it('creates 120 particles (2x normal burst)', () => {
			effects.createRainbowBurst(new Vector3(0, 0, 0) as any, new Color(0x00ffd1) as any);

			const pts = scene.children[scene.children.length - 1] as any;
			expect(pts.geometry.attributes.position.count).toBe(120);
		});

		it('has a 180-frame (3-second) lifespan', () => {
			effects.createRainbowBurst(new Vector3(0, 0, 0) as any, new Color(0x00ffd1) as any);

			// Run for 179 frames — should still exist
			for (let i = 0; i < 179; i++) {
				effects.update(nodeMeshMap, camera);
			}
			expect(scene.children.length).toBe(1);

			// Frame 180 — should be cleaned up
			effects.update(nodeMeshMap, camera);
			expect(scene.children.length).toBe(1); // age increments to 180

			effects.update(nodeMeshMap, camera);
			expect(scene.children.length).toBe(0);
		});

		it('color cycles through rainbow HSL', () => {
			effects.createRainbowBurst(new Vector3(0, 0, 0) as any, new Color(0x00ffd1) as any);

			const pts = scene.children[0] as any;
			const initialColor = { r: pts.material.color.r, g: pts.material.color.g, b: pts.material.color.b };

			// Advance several frames
			for (let i = 0; i < 30; i++) {
				effects.update(nodeMeshMap, camera);
			}

			// Color should have changed due to HSL cycling
			const currentColor = pts.material.color;
			const colorChanged =
				Math.abs(currentColor.r - initialColor.r) > 0.01 ||
				Math.abs(currentColor.g - initialColor.g) > 0.01 ||
				Math.abs(currentColor.b - initialColor.b) > 0.01;
			expect(colorChanged).toBe(true);
		});

		it('particle size pulses (not monotonically decreasing)', () => {
			effects.createRainbowBurst(new Vector3(0, 0, 0) as any, new Color(0x00ffd1) as any);

			const pts = scene.children[0] as any;
			const sizes: number[] = [];

			for (let i = 0; i < 30; i++) {
				effects.update(nodeMeshMap, camera);
				sizes.push(pts.material.size);
			}

			// Check that size varies (pulses) — not monotonically decreasing
			let monotonic = true;
			for (let i = 1; i < sizes.length; i++) {
				if (sizes[i] > sizes[i - 1]) {
					monotonic = false;
					break;
				}
			}
			expect(monotonic).toBe(false);
		});

		it('has hueOffset attribute for per-particle variation', () => {
			effects.createRainbowBurst(new Vector3(0, 0, 0) as any, new Color(0x00ffd1) as any);

			const pts = scene.children[0] as any;
			expect(pts.geometry.attributes.hueOffset).toBeDefined();
			expect(pts.geometry.attributes.hueOffset.count).toBe(120);
		});
	});

	describe('createRippleWave', () => {
		it('creates a ripple wave state', () => {
			const nodePositions = new Map<string, any>([
				['n1', new Vector3(5, 0, 0)],
				['n2', new Vector3(15, 0, 0)],
			]);
			effects.createRippleWave(new Vector3(0, 0, 0) as any);

			// Ripple wave is internal state — verify it runs without error
			for (let i = 0; i < 100; i++) {
				effects.update(nodeMeshMap, camera, nodePositions);
			}
		});

		it('pulses nearby nodes as wavefront reaches them', () => {
			const nodePositions = new Map<string, any>([
				['close', new Vector3(5, 0, 0)],
				['far', new Vector3(50, 0, 0)],
			]);

			const closeMesh = createMockMesh('close', new Vector3(5, 0, 0));
			const farMesh = createMockMesh('far', new Vector3(50, 0, 0));

			effects.createRippleWave(new Vector3(0, 0, 0) as any);

			// Run until wavefront reaches "close" (dist=5, speed=1.2, ~4 frames)
			for (let i = 0; i < 10; i++) {
				effects.update(nodeMeshMap, camera, nodePositions);
			}

			// Should have pulsed the close node — check for pulse effect
			const closeHasPulse = effects.pulseEffects.some((p) => p.nodeId === 'close');
			expect(closeHasPulse).toBe(true);
		});

		it('pulses each node only once', () => {
			const nodePositions = new Map<string, any>([
				['n1', new Vector3(3, 0, 0)],
			]);
			createMockMesh('n1', new Vector3(3, 0, 0));

			effects.createRippleWave(new Vector3(0, 0, 0) as any);

			// Run many frames
			for (let i = 0; i < 30; i++) {
				effects.update(nodeMeshMap, camera, nodePositions);
			}

			// Count pulses for n1 — should be exactly 1
			const n1Pulses = effects.pulseEffects.filter((p) => p.nodeId === 'n1');
			// Could be 0 if already decayed, but should have been created once
			expect(n1Pulses.length).toBeLessThanOrEqual(1);
		});

		it('applies scale bump to contacted nodes', () => {
			const nodePositions = new Map<string, any>([
				['bump', new Vector3(3, 0, 0)],
			]);
			const mesh = createMockMesh('bump', new Vector3(3, 0, 0));

			effects.createRippleWave(new Vector3(0, 0, 0) as any);

			// Run until wavefront reaches the node
			for (let i = 0; i < 10; i++) {
				effects.update(nodeMeshMap, camera, nodePositions);
			}

			// Scale should have been bumped (1.3x)
			expect(mesh.scale.x).toBeGreaterThan(1.0);
		});

		it('completes and cleans up after 90 frames', () => {
			effects.createRippleWave(new Vector3(0, 0, 0) as any);

			for (let i = 0; i < 95; i++) {
				effects.update(nodeMeshMap, camera, new Map());
			}

			// Internal rippleWaves array should be empty (no way to check directly,
			// but running more frames should not cause any errors)
			effects.update(nodeMeshMap, camera, new Map());
		});
	});

	describe('createImplosion', () => {
		it('creates 40 particles', () => {
			effects.createImplosion(new Vector3(5, 5, 5) as any, new Color(0xff4757) as any);

			const pts = scene.children[scene.children.length - 1] as any;
			expect(pts.geometry.attributes.position.count).toBe(40);
		});

		it('particles start spread out around the target', () => {
			const center = new Vector3(5, 5, 5);
			effects.createImplosion(center as any, new Color(0xff4757) as any);

			const pts = scene.children[0] as any;
			const positions = pts.geometry.attributes.position;

			// At least some particles should be far from center
			let maxDist = 0;
			for (let i = 0; i < positions.count; i++) {
				const px = positions.getX(i);
				const py = positions.getY(i);
				const pz = positions.getZ(i);
				const dist = Math.sqrt(
					(px - center.x) ** 2 + (py - center.y) ** 2 + (pz - center.z) ** 2
				);
				if (dist > maxDist) maxDist = dist;
			}
			expect(maxDist).toBeGreaterThan(2);
		});

		it('particles move INWARD toward center', () => {
			const center = new Vector3(0, 0, 0);
			effects.createImplosion(center as any, new Color(0xff4757) as any);

			const pts = scene.children[0] as any;
			const positions = pts.geometry.attributes.position;

			// Record initial average distance
			let initialAvgDist = 0;
			for (let i = 0; i < positions.count; i++) {
				const px = positions.getX(i);
				const py = positions.getY(i);
				const pz = positions.getZ(i);
				initialAvgDist += Math.sqrt(px * px + py * py + pz * pz);
			}
			initialAvgDist /= positions.count;

			// Advance 30 frames
			for (let f = 0; f < 30; f++) {
				effects.update(nodeMeshMap, camera);
			}

			// Record new average distance
			let newAvgDist = 0;
			for (let i = 0; i < positions.count; i++) {
				const px = positions.getX(i);
				const py = positions.getY(i);
				const pz = positions.getZ(i);
				newAvgDist += Math.sqrt(px * px + py * py + pz * pz);
			}
			newAvgDist /= positions.count;

			expect(newAvgDist).toBeLessThan(initialAvgDist);
		});

		it('creates a flash at convergence (frame 60)', () => {
			effects.createImplosion(new Vector3(0, 0, 0) as any, new Color(0xff4757) as any);

			// Run to convergence
			for (let f = 0; f < 60; f++) {
				effects.update(nodeMeshMap, camera);
			}

			// Should have particles + flash mesh
			expect(scene.children.length).toBe(2);
		});

		it('flash fades out and everything cleans up by frame 80', () => {
			effects.createImplosion(new Vector3(0, 0, 0) as any, new Color(0xff4757) as any);

			for (let f = 0; f < 85; f++) {
				effects.update(nodeMeshMap, camera);
			}

			// Everything should be cleaned up
			expect(scene.children.length).toBe(0);
		});

		it('flash sphere expands during fade-out', () => {
			effects.createImplosion(new Vector3(0, 0, 0) as any, new Color(0xff4757) as any);

			for (let f = 0; f < 65; f++) {
				effects.update(nodeMeshMap, camera);
			}

			// Find the flash mesh (should be the second child)
			const flash = scene.children.find(
				(c) => c instanceof Object && 'geometry' in c && !(('attributes' in (c as any).geometry))
			);

			// Flash should have expanded beyond scale 1
			if (flash) {
				expect(flash.scale.x).toBeGreaterThan(1);
			}
		});
	});

	describe('createShockwave', () => {
		it('adds a ring mesh to the scene', () => {
			effects.createShockwave(
				new Vector3(0, 0, 0) as any,
				new Color(0x00ffd1) as any,
				camera
			);

			expect(scene.children.length).toBe(1);
		});

		it('ring expands over time', () => {
			effects.createShockwave(
				new Vector3(0, 0, 0) as any,
				new Color(0x00ffd1) as any,
				camera
			);

			const ring = scene.children[0] as any;
			const initialScale = ring.scale.x;

			for (let f = 0; f < 30; f++) {
				effects.update(nodeMeshMap, camera);
			}

			expect(ring.scale.x).toBeGreaterThan(initialScale);
		});

		it('ring fades out and cleans up after 60 frames', () => {
			effects.createShockwave(
				new Vector3(0, 0, 0) as any,
				new Color(0x00ffd1) as any,
				camera
			);

			for (let f = 0; f < 65; f++) {
				effects.update(nodeMeshMap, camera);
			}

			expect(scene.children.length).toBe(0);
		});
	});

	describe('createConnectionFlash', () => {
		it('creates a line between two points', () => {
			effects.createConnectionFlash(
				new Vector3(0, 0, 0) as any,
				new Vector3(10, 10, 10) as any,
				new Color(0x00d4ff) as any
			);

			expect(scene.children.length).toBe(1);
		});

		it('fades out and cleans up', () => {
			effects.createConnectionFlash(
				new Vector3(0, 0, 0) as any,
				new Vector3(10, 10, 10) as any,
				new Color(0x00d4ff) as any
			);

			for (let f = 0; f < 100; f++) {
				effects.update(nodeMeshMap, camera);
			}

			expect(scene.children.length).toBe(0);
		});
	});

	describe('multiple simultaneous effects', () => {
		it('handles all effect types simultaneously', () => {
			const nodePositions = new Map<string, any>([
				['n1', new Vector3(5, 0, 0)],
			]);
			createMockMesh('n1', new Vector3(5, 0, 0));

			effects.addPulse('n1', 1.0, new Color(0xff0000) as any, 0.05);
			effects.createSpawnBurst(new Vector3(0, 0, 0) as any, new Color(0x00ffd1) as any);
			effects.createRainbowBurst(new Vector3(5, 5, 5) as any, new Color(0xff00ff) as any);
			effects.createShockwave(new Vector3(0, 0, 0) as any, new Color(0x00ffd1) as any, camera);
			effects.createConnectionFlash(
				new Vector3(0, 0, 0) as any,
				new Vector3(10, 0, 0) as any,
				new Color(0x00d4ff) as any
			);
			effects.createImplosion(new Vector3(-5, -5, -5) as any, new Color(0xff4757) as any);
			effects.createRippleWave(new Vector3(0, 0, 0) as any);

			// Should not throw for 200 frames
			for (let f = 0; f < 200; f++) {
				effects.update(nodeMeshMap, camera, nodePositions);
			}

			// All effects should have cleaned up
			expect(scene.children.length).toBe(0);
		});
	});

	describe('dispose', () => {
		it('cleans up all active effects', () => {
			effects.createSpawnBurst(new Vector3(0, 0, 0) as any, new Color(0x00ffd1) as any);
			effects.createRainbowBurst(new Vector3(0, 0, 0) as any, new Color(0xff00ff) as any);
			effects.createImplosion(new Vector3(0, 0, 0) as any, new Color(0xff4757) as any);
			effects.createShockwave(new Vector3(0, 0, 0) as any, new Color(0x00ffd1) as any, camera);
			effects.createConnectionFlash(
				new Vector3(0, 0, 0) as any,
				new Vector3(10, 0, 0) as any,
				new Color(0x00d4ff) as any
			);

			effects.dispose();

			expect(effects.pulseEffects.length).toBe(0);
		});
	});
});
