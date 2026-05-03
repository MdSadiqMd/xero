import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import {
  MARKDOWN_CODE_BLOCK_HIGHLIGHT_BYTE_LIMIT,
  MARKDOWN_SEGMENT_CACHE_MAX_BYTES,
  Markdown,
  getMarkdownSegmentStats,
  resetMarkdownSegmentCacheForTests,
} from './conversation-markdown'

describe('conversation markdown performance behavior', () => {
  it('memoizes fenced segment parsing by message id and text revision', () => {
    resetMarkdownSegmentCacheForTests()
    const text = ['Here is a block:', '```', 'plain code', '```', 'done'].join('\n')
    const { rerender } = render(<Markdown messageId="turn-1" text={text} />)

    rerender(<Markdown messageId="turn-1" text={text} />)
    rerender(<Markdown messageId="turn-1" text={text} />)

    expect(getMarkdownSegmentStats().parses).toBe(1)

    rerender(<Markdown messageId="turn-1" text={`${text}\nstreamed tail`} />)
    expect(getMarkdownSegmentStats().parses).toBe(2)
  })

  it('evicts fenced segment cache entries by retained byte budget', () => {
    resetMarkdownSegmentCacheForTests()
    const largeText = [
      'Here is a block:',
      '```txt',
      'x'.repeat(Math.ceil(MARKDOWN_SEGMENT_CACHE_MAX_BYTES / 5)),
      '```',
    ].join('\n')

    render(<Markdown messageId="turn-large-a" text={largeText} />)
    render(<Markdown messageId="turn-large-b" text={largeText.replace('x', 'y')} />)

    const stats = getMarkdownSegmentStats()
    expect(stats.byteSize).toBeLessThanOrEqual(MARKDOWN_SEGMENT_CACHE_MAX_BYTES)
    expect(stats.evictions).toBeGreaterThan(0)
  })

  it('renders very large code blocks as readable plain text', () => {
    const oversizedCode = 'x'.repeat(MARKDOWN_CODE_BLOCK_HIGHLIGHT_BYTE_LIMIT / 2 + 1)
    render(<Markdown messageId="turn-large" text={['```ts', oversizedCode, '```'].join('\n')} />)

    expect(screen.getByText('Plain')).toBeInTheDocument()
    expect(screen.getByText(oversizedCode)).toBeInTheDocument()
  })
})
