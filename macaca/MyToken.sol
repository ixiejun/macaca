// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/**
 * @title MyToken
 * @dev ERC-20标准代币实现
 * @notice 这是一个符合ERC-20标准的智能合约示例
 */
contract MyToken {
    // ============ 状态变量 ============

    /// @notice 代币名称
    string public constant name = "MyToken";

    /// @notice 代币符号
    string public constant symbol = "MTK";

    /// @notice 代币精度（18位小数）
    uint8 public constant decimals = 18;

    /// @notice 代币总供应量
    uint256 public immutable totalSupply;

    /// @notice 账户余额映射
    mapping(address => uint256) private _balances;

    /// @notice 授权额度映射: owner => (spender => amount)
    mapping(address => mapping(address => uint256)) private _allowances;

    // ============ 事件 ============

    /// @notice 转账事件
    event Transfer(address indexed from, address indexed to, uint256 value);

    /// @notice 授权事件
    event Approval(address indexed owner, address indexed spender, uint256 value);

    // ============ 构造函数 ============

    /**
     * @dev 构造函数 - 初始化代币供应量
     * @notice 100万枚代币，精度18位，即 1,000,000 * 10^18
     */
    constructor() {
        uint256 initialSupply = 1_000_000 * 10 ** uint256(decimals);
        _balances[msg.sender] = initialSupply;
        totalSupply = initialSupply;
        emit Transfer(address(0), msg.sender, initialSupply);
    }

    // ============ 查询函数 ============

    /**
     * @notice 查询账户余额
     * @param account 要查询的账户地址
     * @return 该账户的代币余额
     */
    function balanceOf(address account) external view returns (uint256) {
        return _balances[account];
    }

    /**
     * @notice 查询授权额度
     * @param owner 代币拥有者地址
     * @param spender 被授权者地址
     * @return 剩余可使用的授权额度
     */
    function allowance(address owner, address spender) external view returns (uint256) {
        return _allowances[owner][spender];
    }

    // ============ 转账函数 ============

    /**
     * @notice 转账代币给指定地址
     * @dev 调用者必须有足够的余额
     * @param to 接收代币的地址
     * @param amount 转账数量
     * @return 操作是否成功
     */
    function transfer(address to, uint256 amount) external returns (bool) {
        _transfer(msg.sender, to, amount);
        return true;
    }

    /**
     * @notice 授权第三方使用代币
     * @dev 授权后，spender可以使用transferFrom从调用者账户转账
     * @param spender 被授权的地址
     * @param amount 授权数量
     * @return 操作是否成功
     */
    function approve(address spender, uint256 amount) external returns (bool) {
        _approve(msg.sender, spender, amount);
        return true;
    }

    /**
     * @notice 从授权账户转账代币
     * @dev 调用者必须拥有足够的allowance
     * @param from 代币来源地址
     * @param to 代币目标地址
     * @param amount 转账数量
     * @return 操作是否成功
     */
    function transferFrom(
        address from,
        address to,
        uint256 amount
    ) external returns (bool) {
        // 检查并更新授权额度
        uint256 currentAllowance = _allowances[from][msg.sender];
        require(currentAllowance >= amount, "ERC20: transfer amount exceeds allowance");

        // 使用 unchecked 因为上面已经检查过 currentAllowance >= amount
        unchecked {
            _approve(from, msg.sender, currentAllowance - amount);
        }

        _transfer(from, to, amount);
        return true;
    }

    // ============ 内部函数 ============

    /**
     * @dev 内部转账函数
     * @notice 执行实际的转账逻辑，包含余额检查
     * @param from 发送方地址
     * @param to 接收方地址
     * @param amount 转账数量
     */
    function _transfer(
        address from,
        address to,
        uint256 amount
    ) internal {
        // 验证地址有效性
        require(from != address(0), "ERC20: transfer from the zero address");
        require(to != address(0), "ERC20: transfer to the zero address");

        // 检查发送方余额
        uint256 fromBalance = _balances[from];
        require(fromBalance >= amount, "ERC20: transfer amount exceeds balance");

        // 使用 unchecked 进行减法，因为上面已验证 fromBalance >= amount
        // Solidity 0.8.x 默认有溢出检查，这里可以安全地跳过
        unchecked {
            _balances[from] = fromBalance - amount;
            // 加法不需要 unchecked，因为总供应量不会溢出
            _balances[to] += amount;
        }

        emit Transfer(from, to, amount);
    }

    /**
     * @dev 内部授权函数
     * @notice 设置spender可以使用owner代币的额度
     * @param owner 代币拥有者
     * @param spender 被授权者
     * @param amount 授权额度
     */
    function _approve(
        address owner,
        address spender,
        uint256 amount
    ) internal {
        require(owner != address(0), "ERC20: approve from the zero address");
        require(spender != address(0), "ERC20: approve to the zero address");

        _allowances[owner][spender] = amount;
        emit Approval(owner, spender, amount);
    }

    /**
     * @dev 增加授权额度
     * @notice 安全地增加授权额度，避免授权竞争条件
     * @param spender 被授权者地址
     * @param addedValue 增加的额度
     * @return 操作是否成功
     */
    function increaseAllowance(address spender, uint256 addedValue) external returns (bool) {
        _approve(msg.sender, spender, _allowances[msg.sender][spender] + addedValue);
        return true;
    }

    /**
     * @dev 减少授权额度
     * @notice 安全地减少授权额度
     * @param spender 被授权者地址
     * @param subtractedValue 减少的额度
     * @return 操作是否成功
     */
    function decreaseAllowance(address spender, uint256 subtractedValue) external returns (bool) {
        uint256 currentAllowance = _allowances[msg.sender][spender];
        require(currentAllowance >= subtractedValue, "ERC20: decreased allowance below zero");

        unchecked {
            _approve(msg.sender, spender, currentAllowance - subtractedValue);
        }

        return true;
    }
}
