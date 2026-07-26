import { fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { describe, expect, it, vi } from 'vitest'
import type { Course } from '../types/course'
import ProgressiveCourseList from './ProgressiveCourseList.svelte'

function courses(count: number): Course[] {
	return Array.from({ length: count }, (_, index) => ({
		cd: String(index).padStart(5, '0'),
		nm: `科目 ${index}`,
		prof: '',
		unit: '2',
		dept: '共通',
		campus: '朝倉',
		sem: ['前期'],
		slots: [],
	}))
}

describe('ProgressiveCourseList', () => {
	it('keeps initial rendering bounded and makes every course reachable', async () => {
		const utils = render(ProgressiveCourseList, {
			props: {
				courses: courses(7),
				onselect: vi.fn(),
				initialSize: 2,
				pageSize: 3,
				label: '集中講義・時間未定',
			},
		})
		flushSync()

		expect(utils.container.querySelectorAll('[data-course-card]')).toHaveLength(2)
		const more = utils.getByRole('button', { name: /さらに3件表示（残り5件）/u })
		await fireEvent.click(more)
		flushSync()
		expect(utils.container.querySelectorAll('[data-course-card]')).toHaveLength(5)

		await fireEvent.click(utils.getByRole('button', { name: /さらに2件表示（残り2件）/u }))
		flushSync()
		expect(utils.container.querySelectorAll('[data-course-card]')).toHaveLength(7)
		expect(utils.queryByRole('button', { name: /さらに/u })).not.toBeInTheDocument()
	})

	it('resets its render window when a query replaces the results', async () => {
		const utils = render(ProgressiveCourseList, {
			props: {
				courses: courses(8),
				onselect: vi.fn(),
				initialSize: 2,
				pageSize: 4,
				label: '検索結果',
			},
		})
		await fireEvent.click(utils.getByRole('button', { name: /さらに4件/u }))
		flushSync()
		expect(utils.container.querySelectorAll('[data-course-card]')).toHaveLength(6)

		await utils.rerender({ courses: courses(7) })
		flushSync()
		expect(utils.container.querySelectorAll('[data-course-card]')).toHaveLength(2)
	})
})
