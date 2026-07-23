export function isLeavingTest(currentPage: string, nextPage: string) {
  return currentPage === 'test' && nextPage !== 'test';
}
