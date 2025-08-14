export const formatDate = (date: Date, format: string = 'YYYY-MM-DD'): string => {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  const hours = String(date.getHours()).padStart(2, '0');
  const minutes = String(date.getMinutes()).padStart(2, '0');
  const seconds = String(date.getSeconds()).padStart(2, '0');

  return format
    .replace('YYYY', String(year))
    .replace('MM', month)
    .replace('DD', day)
    .replace('HH', hours)
    .replace('mm', minutes)
    .replace('ss', seconds);
};

export const addDays = (date: Date, days: number): Date => {
  const result = new Date(date);
  result.setDate(result.getDate() + days);
  return result;
};

export const addMonths = (date: Date, months: number): Date => {
  const result = new Date(date);
  result.setMonth(result.getMonth() + months);
  return result;
};

export const addYears = (date: Date, years: number): Date => {
  const result = new Date(date);
  result.setFullYear(result.getFullYear() + years);
  return result;
};

export const diffInDays = (date1: Date, date2: Date): number => {
  const timeDiff = Math.abs(date1.getTime() - date2.getTime());
  return Math.ceil(timeDiff / (1000 * 3600 * 24));
};

export const diffInHours = (date1: Date, date2: Date): number => {
  const timeDiff = Math.abs(date1.getTime() - date2.getTime());
  return Math.ceil(timeDiff / (1000 * 3600));
};

export const diffInMinutes = (date1: Date, date2: Date): number => {
  const timeDiff = Math.abs(date1.getTime() - date2.getTime());
  return Math.ceil(timeDiff / (1000 * 60));
};

export const startOfDay = (date: Date): Date => {
  const result = new Date(date);
  result.setHours(0, 0, 0, 0);
  return result;
};

export const endOfDay = (date: Date): Date => {
  const result = new Date(date);
  result.setHours(23, 59, 59, 999);
  return result;
};

export const isWeekend = (date: Date): boolean => {
  const day = date.getDay();
  return day === 0 || day === 6;
};

export const getWeekNumber = (date: Date): number => {
  const startOfYear = new Date(date.getFullYear(), 0, 1);
  const days = Math.floor((date.getTime() - startOfYear.getTime()) / (24 * 60 * 60 * 1000));
  return Math.ceil((days + startOfYear.getDay() + 1) / 7);
};

export default {
  formatDate,
  addDays,
  addMonths,
  addYears,
  diffInDays,
  diffInHours,
  diffInMinutes,
  startOfDay,
  endOfDay,
  isWeekend,
  getWeekNumber
};
