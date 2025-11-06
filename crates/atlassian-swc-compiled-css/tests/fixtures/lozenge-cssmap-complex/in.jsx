import { cssMap } from '@compiled/react';

const styles = cssMap({
  container: {
    display: 'inline-flex',
    boxSizing: 'border-box',
    position: 'static',
    blockSize: 'min-content',
    borderRadius: '3px',
    overflow: 'hidden',
    paddingInlineStart: '4px',
    paddingInlineEnd: '4px',
  },
  containerSubtle: {
    outlineOffset: -1,
  },
  text: {
    fontFamily: 'ui-sans-serif',
    fontSize: '11px',
    fontStyle: 'normal',
    fontWeight: 'bold',
    lineHeight: '16px',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    textTransform: 'uppercase',
    whiteSpace: 'nowrap',
  },
  customLetterspacing: {
    letterSpacing: 0.165,
  },
  bgBoldDefault: { backgroundColor: '#DDDEE1' },
  bgBoldInprogress: { backgroundColor: '#8FB8F6' },
  bgBoldMoved: { backgroundColor: '#F9C84E' },
  bgBoldNew: { backgroundColor: '#D8A0F7' },
  bgBoldRemoved: { backgroundColor: '#FD9891' },
  bgBoldSuccess: { backgroundColor: '#B3DF72' },
  bgSubtleDefault: { backgroundColor: '#F4F5F7' },
  bgSubtleInprogress: { backgroundColor: '#F4F5F7' },
  bgSubtleMoved: { backgroundColor: '#F4F5F7' },
  bgSubtleNew: { backgroundColor: '#F4F5F7' },
  bgSubtleRemoved: { backgroundColor: '#F4F5F7' },
  bgSubtleSuccess: { backgroundColor: '#F4F5F7' },
  borderSubtleDefault: { border: '1px solid #B7B9BE' },
  borderSubtleInprogress: { border: '1px solid #669DF1' },
  borderSubtleMoved: { border: '1px solid #FCA700' },
  borderSubtleNew: { border: '1px solid #C97CF4' },
  borderSubtleRemoved: { border: '1px solid #F87168' },
  borderSubtleSuccess: { border: '1px solid #94C748' },
  textSubtle: { color: '#172B4D' },
  textBold: { color: '#292A2E' },
});

function Lozenge({ children, isBold = false, appearance = 'default' }) {
  const appearanceStyle = isBold ? 'Bold' : 'Subtle';
  const bgClass = `bg${appearanceStyle}${appearance.charAt(0).toUpperCase() + appearance.slice(1)}`;
  const textClass = `text${appearanceStyle}`;
  
  return (
    <span className={styles.container()}>
      <span className={styles[bgClass]()}>
        <span className={styles.text()}>
          <span className={styles.customLetterspacing()}>
            <span className={styles[textClass]()}>
              {children}
            </span>
          </span>
        </span>
      </span>
    </span>
  );
}

export default Lozenge;