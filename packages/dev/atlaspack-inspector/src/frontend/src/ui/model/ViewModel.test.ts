import {ViewModel} from './ViewModel';

describe('ViewModel', () => {
  describe('ViewModel::groupsById', () => {
    it('should return a map of groups by id', () => {
      const viewModel = new ViewModel();
      viewModel.data = {
        groups: [
          {
            id: '1',
            type: 'bundle',
            label: 'Bundle 1',
            weight: 1,
            groups: [],
          },
          {
            id: '2',
            type: 'bundle',
            label: 'Bundle 2',
            weight: 2,
            groups: [],
          },
        ],
      };

      expect(viewModel.groupsById.size).toBe(2);
      expect(viewModel.groupsById.get('1')?.label).toBe('Bundle 1');
      expect(viewModel.groupsById.get('2')?.label).toBe('Bundle 2');
    });
  });
});
