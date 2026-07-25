import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { settingsRegistry, defineModuleSettings } from '@/modules/settings/registry'

beforeEach(() => settingsRegistry.clear())
afterEach(() => settingsRegistry.clear())

describe('SettingsRegistry', () => {
  it('注册模块后 getAllGroups 返回对应分组', () => {
    defineModuleSettings({
      moduleId: 'test',
      groups: [{ id: 'g1', label: '分组1', items: [] }],
    })
    expect(settingsRegistry.getAllGroups()).toHaveLength(1)
  })

  it('同一 moduleId 会被覆盖', () => {
    defineModuleSettings({ moduleId: 'test', groups: [{ id: 'g1', label: 'A', items: [] }] })
    defineModuleSettings({ moduleId: 'test', groups: [{ id: 'g2', label: 'B', items: [] }] })
    expect(settingsRegistry.getAllGroups()[0].id).toBe('g2')
  })

  it('getAllKeys 收集所有 item key', () => {
    defineModuleSettings({
      moduleId: 'test',
      groups: [{
        id: 'g1', label: 'G',
        items: [
          { key: 'a', type: 'boolean', label: 'A', default: true },
          { key: 'b', type: 'string', label: 'B', default: '' },
        ],
      }],
    })
    expect(settingsRegistry.getAllKeys()).toEqual(['a', 'b'])
  })

  it('findItemDef 查找正确', () => {
    defineModuleSettings({
      moduleId: 'test',
      groups: [{
        id: 'g1', label: 'G',
        items: [{ key: 'mySetting', type: 'boolean', label: 'My Setting', default: false }],
      }],
    })
    const f = settingsRegistry.findItemDef('mySetting')
    expect(f?.item.label).toBe('My Setting')
  })

  it('findItemDef 不存在返回 null', () => {
    expect(settingsRegistry.findItemDef('nope')).toBeNull()
  })

  it('order 排序', () => {
    defineModuleSettings({ moduleId: 'last', order: 100, groups: [{ id: 'later', label: 'L', items: [] }] })
    defineModuleSettings({ moduleId: 'first', order: 0, groups: [{ id: 'earlier', label: 'E', items: [] }] })
    expect(settingsRegistry.getAllGroups()[0].id).toBe('earlier')
  })
})
