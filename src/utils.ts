export function extractComicId(input: string): number | undefined {
  // 如果是数字，直接返回
  const comicId = parseInt(input)
  if (!isNaN(comicId)) {
    return comicId
  }
  // 否则需要从链接中提取
  const regex = /aid-(\d+)/
  const match = input.match(regex)
  if (match === null || match[1] === null) {
    return
  }
  return parseInt(match[1])
}
