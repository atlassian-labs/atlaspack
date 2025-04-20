import React from 'react';

interface TestProps {
  text: string;
}

export function Test({ text }: TestProps) {
  return <div>{text}</div>;
}
