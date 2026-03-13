// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/**
 * @title ERC20Token
 * @dev Implementation of the ERC-20 standard token
 */
contract ERC20Token {
    // Token metadata
    string public name;
    string public symbol;
    uint8 public decimals;

    // Token state
    uint256 private _totalSupply;
    mapping(address => uint256) private _balances;
    mapping(address => mapping(address => uint256)) private _allowances;

    // Events
    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);

    /**
     * @dev Constructor that initializes the token with name, symbol, decimals and total supply
     * @param tokenName The name of the token
     * @param tokenSymbol The symbol of the token
     * @param tokenDecimals The number of decimals for the token
     * @param initialSupply The initial total supply (in wei units)
     */
    constructor(
        string memory tokenName,
        string memory tokenSymbol,
        uint8 tokenDecimals,
        uint256 initialSupply
    ) {
        require(initialSupply > 0, "Initial supply must be greater than 0");

        name = tokenName;
        symbol = tokenSymbol;
        decimals = tokenDecimals;

        // Mint all tokens to the deployer
        _totalSupply = initialSupply;
        _balances[msg.sender] = initialSupply;

        emit Transfer(address(0), msg.sender, initialSupply);
    }

    /**
     * @dev Returns the total supply of the token
     */
    function totalSupply() external view returns (uint256) {
        return _totalSupply;
    }

    /**
     * @dev Returns the balance of a specific account
     * @param account The address to query the balance of
     */
    function balanceOf(address account) external view returns (uint256) {
        return _balances[account];
    }

    /**
     * @dev Transfers tokens from the caller to a specified address
     * @param to The address to transfer tokens to
     * @param amount The amount of tokens to transfer
     * @return success Boolean indicating whether the transfer was successful
     */
    function transfer(address to, uint256 amount) external returns (bool success) {
        _transfer(msg.sender, to, amount);
        return true;
    }

    /**
     * @dev Transfers tokens from one address to another (requires approval)
     * @param from The address to transfer tokens from
     * @param to The address to transfer tokens to
     * @param amount The amount of tokens to transfer
     * @return success Boolean indicating whether the transfer was successful
     */
    function transferFrom(
        address from,
        address to,
        uint256 amount
    ) external returns (bool success) {
        require(amount <= _allowances[from][msg.sender], "Allowance exceeded");

        _transfer(from, to, amount);

        // Deduct allowance (no underflow due to require check above)
        unchecked {
            _allowances[from][msg.sender] -= amount;
        }

        return true;
    }

    /**
     * @dev Approves a spender to spend a certain amount of tokens on behalf of the caller
     * @param spender The address that will be approved to spend tokens
     * @param amount The amount of tokens to allow the spender to spend
     * @return success Boolean indicating whether the approval was successful
     */
    function approve(address spender, uint256 amount) external returns (bool success) {
        require(spender != address(0), "Cannot approve zero address");

        _allowances[msg.sender][spender] = amount;
        emit Approval(msg.sender, spender, amount);

        return true;
    }

    /**
     * @dev Returns the remaining number of tokens that a spender is allowed to spend
     * @param owner The address that owns the tokens
     * @param spender The address that is allowed to spend tokens
     */
    function allowance(
        address owner,
        address spender
    ) external view returns (uint256 remaining) {
        return _allowances[owner][spender];
    }

    /**
     * @dev Internal function to handle token transfers
     * @param from The address to transfer tokens from
     * @param to The address to transfer tokens to
     * @param amount The amount of tokens to transfer
     */
    function _transfer(address from, address to, uint256 amount) internal {
        require(from != address(0), "Transfer from zero address");
        require(to != address(0), "Transfer to zero address");
        require(amount > 0, "Transfer amount must be greater than 0");
        require(_balances[from] >= amount, "Insufficient balance");

        // Deduct from sender
        _balances[from] -= amount;

        // Add to receiver
        _balances[to] += amount;

        emit Transfer(from, to, amount);
    }
}
